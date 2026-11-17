//! Group as Mesh's optimizer: the parts' triangles made as few as they can
//! be without changing anything that can be seen (silhouette, shading,
//! textures).
//!
//! The passes, in order:
//!
//! 1. **Weld**: corners closer than a hair (a millionth of the group's
//!    size) become one point, and vertices at one point with the same
//!    normal and uv become one vertex. Vertices whose normal or uv differ
//!    (a hard edge, a texture seam, another look) stay apart.
//! 2. **Degenerate and duplicate triangles** go: ones with no area, and a
//!    second copy of a triangle (same corners, facing the same way, same
//!    look).
//! 3. **Pressed together**: a triangle with an exact twin facing the other
//!    way in another, solid part is pressed against it, so it can't be
//!    seen. So is one that faces of other solid parts in its plane,
//!    facing it, cover all of between them (a brick's bottom across two
//!    bricks below: the faces are cut out of it in the plane, and nothing
//!    may be left). One only partly covered is kept whole.
//! 4. **Hidden inside**: a triangle whose seen side (both sides, for a
//!    two-sided look) lies wholly inside another solid part is gone. A
//!    point a hair in front of it must be inside that part (a ray counts
//!    the part's surfaces it crosses, with exact tie-breaking), and the
//!    triangle, moved that hair forward and in from its edges, must not
//!    cross any of the part's triangles. Only *solid* parts hide anything:
//!    closed (every edge has a face on both sides), with room inside, and
//!    opaque all over. Open meshes (planes, leaves) are grouped as they are
//!    and never hide anything.
//! 5. **Coplanar merge**: touching triangles in one plane with one look and
//!    one normal are filled again from their outline, without the corners
//!    inside it or in a straight line along it, and without needle
//!    triangles (`merge::merge_flat_faces_with`); a corner another face
//!    needs is kept on both sides, so no T-junction opens a crack. A
//!    textured face only joins where its uvs are one flat mapping across
//!    the pieces, so the texture lands where it did. Plain faces first
//!    take the corners of faces beside them that lie on their edges (a
//!    brick's corner halfway along the top of the brick below), so faces
//!    that only met at T-junctions share edges and join too; that's kept
//!    only when it ends with fewer triangles.
//! 6. Unused vertices are dropped.
//!
//! Smooth surfaces (a corner's normals not all the same) are never filled
//! again, so their shading can't change.

use std::collections::HashMap;

use glam::{DVec2, DVec3, IVec3, Vec3};

use crate::bvh::Bvh;
use crate::merge::merge_flat_faces_with;
use crate::{Mesh, Surface, Triangle, position_key};

/// Corners this close, as a fraction of the group's size, are welded.
const WELD: f32 = 1e-6;

/// How far in front of a triangle (and in from its edges) the hidden test
/// looks, as a fraction of the group's size. Far above f32's rounding,
/// far below anything that could be seen.
const GAP: f64 = 4e-6;

/// What the optimizer knows about one of the parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartInfo {
    /// Closed, with room inside, and opaque all over: what's inside it, or
    /// pressed against it, can't be seen.
    pub solid: bool,
}

/// What the optimizer knows about one look (a `Surface::Outer` index).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LookInfo {
    /// No texture at all: its flat faces can be filled again however
    /// their uvs run.
    pub plain: bool,
    /// Both sides of its triangles are drawn.
    pub double_sided: bool,
}

/// How much each pass took away.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OptimizeReport {
    /// Vertices made one with another.
    pub welded: usize,
    /// Triangles with no area.
    pub degenerate: usize,
    /// Second copies of a triangle.
    pub duplicates: usize,
    /// Triangles pressed against a solid part's.
    pub pressed: usize,
    /// Triangles inside a solid part.
    pub hidden: usize,
    /// Triangles fewer once flat faces were filled again.
    pub merged: usize,
}

/// `mesh` (every part's triangles in one space, each triangle's look in
/// its `Surface::Outer` and its part in `part_of`) with as few triangles
/// as look the same. See the module's passes.
pub fn optimize(
    mesh: &Mesh,
    part_of: &[u32],
    parts: &[PartInfo],
    looks: &[LookInfo],
) -> (Mesh, OptimizeReport) {
    let mut report = OptimizeReport::default();
    let mut mesh = mesh.clone();
    let mut part_of = part_of.to_vec();
    part_of.resize(mesh.triangles.len(), u32::MAX);
    let (min, max) = mesh.bounds();
    let extent = (max - min)
        .max_element()
        .max(min.abs().max(max.abs()).max_element())
        .max(1.0);

    // 1. Weld.
    let vertices = used_vertices(&mesh);
    weld(&mut mesh, extent * WELD);
    report.welded = vertices.saturating_sub(used_vertices(&mesh));

    // 2. Degenerate and duplicate triangles.
    let degenerate: Vec<bool> = (0..mesh.triangles.len())
        .map(|t| is_degenerate(&mesh, t))
        .collect();
    report.degenerate = degenerate.iter().filter(|&&gone| gone).count();
    retain(&mut mesh, &mut part_of, &degenerate);
    let duplicate = duplicates(&mesh);
    report.duplicates = duplicate.iter().filter(|&&gone| gone).count();
    retain(&mut mesh, &mut part_of, &duplicate);

    // The solid parts as they are now, for both tests that follow.
    let solid = |part: u32| parts.get(part as usize).is_some_and(|info| info.solid);
    let solids = Solids::new(&mesh, &part_of, &solid);

    // 3. Pressed together.
    let pressed = pressed(&mesh, &part_of, &solid);
    report.pressed = pressed.iter().filter(|&&gone| gone).count();
    retain(&mut mesh, &mut part_of, &pressed);

    // ... and pressed against solid parts' faces that cover it between
    // them.
    let tolerance = f64::from(extent * WELD) * 4.0;
    let covered = covered(&mesh, &part_of, &solid, tolerance);
    report.pressed += covered.iter().filter(|&&gone| gone).count();
    retain(&mut mesh, &mut part_of, &covered);

    // 4. Hidden inside.
    let gap = f64::from(extent) * GAP;
    let hidden = solids.hidden(&mesh, &part_of, looks, gap);
    report.hidden = hidden.iter().filter(|&&gone| gone).count();
    retain(&mut mesh, &mut part_of, &hidden);

    // 5. Coplanar merge.
    let plain = |surface: Surface| match surface {
        Surface::Outer(index) => looks.get(index as usize).is_some_and(|look| look.plain),
        Surface::Interior => false,
    };
    let before = mesh.triangles.len();
    let merged = merge_flat_faces_with(&mesh, &plain, true);
    // Plain faces' corners lying on another's edge (a brick's corner on the
    // brick below's top edge) split that edge first, so the two faces share
    // it and become one. Kept only if it ends with fewer triangles.
    let split = split_t_junctions(&mesh, &plain, tolerance);
    let merged = match split {
        Some(split) => {
            let joined = merge_flat_faces_with(&split, &plain, true);
            if joined.triangles.len() < merged.triangles.len() {
                joined
            } else {
                merged
            }
        }
        None => merged,
    };
    report.merged = before.saturating_sub(merged.triangles.len());

    // 6. Only the vertices used.
    (compact(&merged), report)
}

/// How many vertices its triangles use.
pub(crate) fn used_vertices(mesh: &Mesh) -> usize {
    let mut used = vec![false; mesh.positions.len()];
    for triangle in &mesh.triangles {
        for corner in triangle.corners {
            used[corner as usize] = true;
        }
    }
    used.iter().filter(|&&used| used).count()
}

/// Moves every corner within `tolerance` of an earlier one onto it, then
/// makes vertices at one point with the same normal and uv one vertex.
fn weld(mesh: &mut Mesh, tolerance: f32) {
    let tolerance = tolerance.max(f32::MIN_POSITIVE);
    let cell = |p: Vec3| (p / tolerance).floor().as_ivec3();
    // The points kept, by grid cell.
    let mut grid: HashMap<IVec3, Vec<u32>> = HashMap::new();
    let mut points: Vec<Vec3> = Vec::new();
    let mut point_of = Vec::with_capacity(mesh.positions.len());
    for &position in &mesh.positions {
        let found = position.is_finite().then(|| {
            let at = cell(position);
            let mut best: Option<u32> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(list) = grid.get(&(at + IVec3::new(dx, dy, dz))) else {
                            continue;
                        };
                        for &point in list {
                            let near = (points[point as usize] - position).abs().max_element()
                                <= tolerance;
                            // The earliest point wins, whatever order the
                            // cells are looked in.
                            if near && best.is_none_or(|best| point < best) {
                                best = Some(point);
                            }
                        }
                    }
                }
            }
            best
        });
        let point = match found.flatten() {
            Some(point) => point,
            None => {
                points.push(position);
                let point = (points.len() - 1) as u32;
                if position.is_finite() {
                    grid.entry(cell(position)).or_default().push(point);
                }
                point
            }
        };
        point_of.push(point);
    }

    // Vertices by point, normal and uv (both on a fine grid, so ones a
    // rounding apart still match).
    type Key = (u32, [i32; 3], [i64; 2]);
    let key = |vertex: usize| -> Key {
        let normal = (mesh.normals[vertex] * 1e5).round().as_ivec3().to_array();
        let uv = mesh.uvs[vertex].as_dvec2() * 1e6;
        (
            point_of[vertex],
            normal,
            [uv.x.round() as i64, uv.y.round() as i64],
        )
    };
    let mut welded = Mesh::default();
    let mut vertex_of: HashMap<Key, u32> = HashMap::new();
    let mut new_index = vec![u32::MAX; mesh.positions.len()];
    for triangle in &mut mesh.triangles {
        for corner in &mut triangle.corners {
            let old = *corner as usize;
            if new_index[old] == u32::MAX {
                new_index[old] = *vertex_of.entry(key(old)).or_insert_with(|| {
                    welded.push_vertex(
                        points[point_of[old] as usize],
                        mesh.normals[old],
                        mesh.uvs[old],
                    )
                });
            }
            *corner = new_index[old];
        }
    }
    welded.triangles = std::mem::take(&mut mesh.triangles);
    *mesh = welded;
}

/// True if triangle `t` has no area: two corners at one point, or all
/// three in a line.
fn is_degenerate(mesh: &Mesh, t: usize) -> bool {
    let [a, b, c] = mesh.triangles[t]
        .corners
        .map(|corner| mesh.positions[corner as usize]);
    if a == b || b == c || c == a {
        return true;
    }
    let [a, b, c] = [a, b, c].map(Vec3::as_dvec3);
    let longest = (b - a)
        .length_squared()
        .max((c - b).length_squared())
        .max((a - c).length_squared());
    (b - a).cross(c - a).length_squared() <= 1e-20 * longest * longest
}

/// Triangle `t`'s corners as positions, turned so the smallest comes
/// first (the same triangle facing the same way gives the same key).
fn corner_key(mesh: &Mesh, t: usize, flipped: bool) -> [[u32; 3]; 3] {
    let [a, b, c] = mesh.triangles[t]
        .corners
        .map(|corner| position_key(mesh.positions[corner as usize]));
    let [a, b, c] = if flipped { [a, c, b] } else { [a, b, c] };
    if a <= b && a <= c {
        [a, b, c]
    } else if b <= a && b <= c {
        [b, c, a]
    } else {
        [c, a, b]
    }
}

/// Marks each triangle that's a second copy of an earlier one with the
/// same look.
fn duplicates(mesh: &Mesh) -> Vec<bool> {
    let mut seen: HashMap<([[u32; 3]; 3], Surface), ()> = HashMap::new();
    (0..mesh.triangles.len())
        .map(|t| {
            seen.insert((corner_key(mesh, t, false), mesh.triangles[t].surface), ())
                .is_some()
        })
        .collect()
}

/// Marks each triangle with an exact twin facing the other way in another
/// part that's solid: the two are pressed together.
fn pressed(mesh: &Mesh, part_of: &[u32], solid: &dyn Fn(u32) -> bool) -> Vec<bool> {
    let mut by_corners: HashMap<[[u32; 3]; 3], Vec<usize>> = HashMap::new();
    for t in 0..mesh.triangles.len() {
        by_corners
            .entry(corner_key(mesh, t, false))
            .or_default()
            .push(t);
    }
    (0..mesh.triangles.len())
        .map(|t| {
            by_corners
                .get(&corner_key(mesh, t, true))
                .is_some_and(|twins| {
                    twins
                        .iter()
                        .any(|&u| part_of[u] != part_of[t] && solid(part_of[u]))
                })
        })
        .collect()
}

/// Keeps the triangles (and their parts) not marked `gone`.
fn retain(mesh: &mut Mesh, part_of: &mut Vec<u32>, gone: &[bool]) {
    let mut k = 0;
    mesh.triangles.retain(|_| {
        k += 1;
        !gone[k - 1]
    });
    let mut k = 0;
    part_of.retain(|_| {
        k += 1;
        !gone[k - 1]
    });
}

/// `mesh` with only the vertices its triangles use, in the order first
/// used.
pub(crate) fn compact(mesh: &Mesh) -> Mesh {
    let mut compact = Mesh::default();
    let mut index = vec![u32::MAX; mesh.positions.len()];
    for triangle in &mesh.triangles {
        let corners = triangle.corners.map(|corner| {
            let old = corner as usize;
            if index[old] == u32::MAX {
                index[old] =
                    compact.push_vertex(mesh.positions[old], mesh.normals[old], mesh.uvs[old]);
            }
            index[old]
        });
        compact.triangles.push(Triangle {
            corners,
            surface: triangle.surface,
        });
    }
    compact
}

/// One solid part's triangles, with a hierarchy of boxes to find the ones
/// near a point quickly.
struct SolidPart {
    part: u32,
    min: Vec3,
    max: Vec3,
    triangles: Vec<[u32; 3]>,
    bvh: Bvh,
}

/// Every solid part, and the positions their triangles index.
struct Solids {
    positions: Vec<Vec3>,
    parts: Vec<SolidPart>,
}

impl Solids {
    fn new(mesh: &Mesh, part_of: &[u32], solid: &dyn Fn(u32) -> bool) -> Solids {
        let mut by_part: Vec<(u32, Vec<[u32; 3]>)> = Vec::new();
        let mut index_of: HashMap<u32, usize> = HashMap::new();
        for (t, triangle) in mesh.triangles.iter().enumerate() {
            let part = part_of[t];
            if !solid(part) {
                continue;
            }
            let index = *index_of.entry(part).or_insert_with(|| {
                by_part.push((part, Vec::new()));
                by_part.len() - 1
            });
            by_part[index].1.push(triangle.corners);
        }
        let positions = mesh.positions.clone();
        let parts = by_part
            .into_iter()
            .map(|(part, triangles)| {
                let (min, max) = triangles.iter().flatten().fold(
                    (Vec3::INFINITY, Vec3::NEG_INFINITY),
                    |(min, max), &v| {
                        let p = positions[v as usize];
                        (min.min(p), max.max(p))
                    },
                );
                let bvh = Bvh::build(&positions, &triangles);
                SolidPart {
                    part,
                    min,
                    max,
                    triangles,
                    bvh,
                }
            })
            .collect();
        Solids { positions, parts }
    }

    /// Marks each triangle whose seen sides are wholly inside a solid part
    /// other than its own.
    fn hidden(&self, mesh: &Mesh, part_of: &[u32], looks: &[LookInfo], gap: f64) -> Vec<bool> {
        let mut scratch = Vec::new();
        (0..mesh.triangles.len())
            .map(|t| {
                if self.parts.is_empty() {
                    return false;
                }
                let triangle = &mesh.triangles[t];
                let corners = triangle
                    .corners
                    .map(|corner| mesh.positions[corner as usize].as_dvec3());
                let [a, b, c] = corners;
                let Some(normal) = (b - a).cross(c - a).try_normalize() else {
                    return false;
                };
                let two_sided = match triangle.surface {
                    Surface::Outer(index) => looks
                        .get(index as usize)
                        .is_some_and(|look| look.double_sided),
                    Surface::Interior => false,
                };
                let sides: &[f64] = if two_sided { &[1.0, -1.0] } else { &[1.0] };
                sides.iter().all(|&side| {
                    let moved = moved_in(corners, normal * side, gap);
                    let [a, b, c] = moved;
                    let (low, high) = (a.min(b).min(c).as_vec3(), a.max(b).max(c).as_vec3());
                    self.parts.iter().any(|solid| {
                        solid.part != part_of[t]
                            && solid.min.cmple(low).all()
                            && solid.max.cmpge(high).all()
                            && self.encloses(solid, moved, &mut scratch)
                    })
                })
            })
            .collect()
    }

    /// True if the triangle `corners` is wholly inside `solid`: none of its
    /// triangles cross it, and its middle and corners are inside.
    fn encloses(&self, solid: &SolidPart, corners: [DVec3; 3], scratch: &mut Vec<u32>) -> bool {
        let [a, b, c] = corners;
        let (low, high) = (a.min(b).min(c), a.max(b).max(c));
        scratch.clear();
        solid.bvh.boxed(
            low.as_vec3() - Vec3::splat(1e-6),
            high.as_vec3() + Vec3::splat(1e-6),
            scratch,
        );
        let at = |v: u32| self.positions[v as usize].as_dvec3();
        for &t in scratch.iter() {
            let other = solid.triangles[t as usize].map(at);
            if triangles_touch(corners, other) {
                return false;
            }
        }
        let middle = (a + b + c) / 3.0;
        [middle, a, b, c]
            .into_iter()
            .all(|point| self.inside(solid, point, scratch))
    }

    /// True if `point` is inside `solid`: a ray from it along +X crosses
    /// its surface an odd number of times. The ray's crossings are worked
    /// out exactly on the plane across X (f32 corners, f64 sums), with
    /// each edge and corner belonging to exactly one of the triangles
    /// beside it, so a ray through an edge counts once.
    fn inside(&self, solid: &SolidPart, point: DVec3, scratch: &mut Vec<u32>) -> bool {
        let q = point.as_vec3();
        let flat_q = DVec2::new(f64::from(q.y), f64::from(q.z));
        scratch.clear();
        solid
            .bvh
            .boxed(q, Vec3::new(f32::INFINITY, q.y, q.z), scratch);
        let mut crossings = 0;
        for &t in scratch.iter() {
            let [a, b, c] = solid.triangles[t as usize].map(|v| self.positions[v as usize]);
            let flat = |p: Vec3| DVec2::new(f64::from(p.y), f64::from(p.z));
            let (fa, mut fb, mut fc) = (flat(a), flat(b), flat(c));
            let (xa, mut xb, mut xc) = (f64::from(a.x), f64::from(b.x), f64::from(c.x));
            let area = orient(fa, fb, fc);
            if area == 0.0 {
                continue;
            }
            if area < 0.0 {
                std::mem::swap(&mut fb, &mut fc);
                std::mem::swap(&mut xb, &mut xc);
            }
            let covers = |from: DVec2, to: DVec2, weight: f64| {
                weight > 0.0 || (weight == 0.0 && owns(from, to))
            };
            let (wa, wb, wc) = (
                orient(fb, fc, flat_q),
                orient(fc, fa, flat_q),
                orient(fa, fb, flat_q),
            );
            if !(covers(fb, fc, wa) && covers(fc, fa, wb) && covers(fa, fb, wc)) {
                continue;
            }
            let x = (wa * xa + wb * xb + wc * xc) / area.abs();
            if x > f64::from(q.x) {
                crossings += 1;
            }
        }
        crossings % 2 == 1
    }
}

/// Twice the signed area of `a b c` (positive counter-clockwise). With f32
/// corners, the sign is exact: the differences and products are exact in
/// f64, and rounding their difference never changes its sign.
fn orient(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Whether the edge `from → to` of a counter-clockwise triangle owns the
/// points on it: its left edges and top edges do (as when filling
/// triangles on screen), so of two triangles side by side exactly one
/// does.
fn owns(from: DVec2, to: DVec2) -> bool {
    to.y < from.y || (to.y == from.y && to.x < from.x)
}

/// The triangle `corners` moved `gap` along `toward` (a unit normal) and
/// `gap` in from each corner toward its middle (at most a quarter of the
/// way), so a face flush with another's edge doesn't touch it.
fn moved_in(corners: [DVec3; 3], toward: DVec3, gap: f64) -> [DVec3; 3] {
    let middle = (corners[0] + corners[1] + corners[2]) / 3.0;
    corners.map(|corner| {
        let inward = middle - corner;
        let length = inward.length();
        let step = if length > 0.0 {
            inward * (gap.min(length / 4.0) / length)
        } else {
            DVec3::ZERO
        };
        corner + step + toward * gap
    })
}

/// True if the triangles `a` and `b` cross or touch (an edge of either
/// passes through the other). One lying in the other's plane counts as
/// touching: the test errs toward keeping a triangle.
fn triangles_touch(a: [DVec3; 3], b: [DVec3; 3]) -> bool {
    let edges = |t: [DVec3; 3]| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
    edges(a)
        .into_iter()
        .any(|(from, to)| segment_touches(from, to, b))
        || edges(b)
            .into_iter()
            .any(|(from, to)| segment_touches(from, to, a))
}

/// True if the segment `from → to` passes through (or touches) the
/// triangle `t` (Möller and Trumbore, with a little room on every side).
fn segment_touches(from: DVec3, to: DVec3, [a, b, c]: [DVec3; 3]) -> bool {
    const ROOM: f64 = 1e-9;
    let direction = to - from;
    let (e1, e2) = (b - a, c - a);
    let h = direction.cross(e2);
    let det = e1.dot(h);
    let scale = e1.length() * e2.length() * direction.length();
    if scale == 0.0 {
        return false;
    }
    if det.abs() <= 1e-12 * scale {
        // Parallel: touching only if it lies in the triangle's plane.
        let normal = e1.cross(e2);
        let length = normal.length();
        if length == 0.0 {
            return false;
        }
        let distance = (from - a).dot(normal) / length;
        return distance.abs() <= ROOM * (e1.length() + e2.length());
    }
    let f = 1.0 / det;
    let s = from - a;
    let u = f * s.dot(h);
    if !(-ROOM..=1.0 + ROOM).contains(&u) {
        return false;
    }
    let q = s.cross(e1);
    let v = f * direction.dot(q);
    if v < -ROOM || u + v > 1.0 + ROOM {
        return false;
    }
    let t = f * e2.dot(q);
    (-ROOM..=1.0 + ROOM).contains(&t)
}

/// Splits the edges of plain flat triangles at the corners of other plain
/// flat triangles in their plane, with their look, lying on them (within
/// `tolerance`): T-junctions become shared edges. Only edges on a face's
/// outline (no triangle uses them the other way) and corners on outlines
/// are looked at. `None` when there's nothing to split.
fn split_t_junctions(mesh: &Mesh, plain: &dyn Fn(Surface) -> bool, tolerance: f64) -> Option<Mesh> {
    let grid = |normal: Vec3| (normal * 1e6).round().as_ivec3().to_array();
    // Plain flat triangles by look and plane.
    type PlaneKey = (Surface, [i32; 3], i64);
    let mut planes: HashMap<PlaneKey, Vec<usize>> = HashMap::new();
    let mut plane_order: Vec<PlaneKey> = Vec::new();
    for (t, triangle) in mesh.triangles.iter().enumerate() {
        if !plain(triangle.surface) {
            continue;
        }
        let [a, b, c] = triangle.corners.map(|k| grid(mesh.normals[k as usize]));
        if a != b || b != c || a == [0; 3] {
            continue;
        }
        let normal = mesh.normals[triangle.corners[0] as usize].as_dvec3();
        let offset = normal.dot(mesh.positions[triangle.corners[0] as usize].as_dvec3());
        let key = (
            triangle.surface,
            a,
            (offset / (tolerance * 4.0).max(f64::MIN_POSITIVE)).round() as i64,
        );
        planes
            .entry(key)
            .or_insert_with(|| {
                plane_order.push(key);
                Vec::new()
            })
            .push(t);
    }

    let at = |vertex: u32| mesh.positions[vertex as usize];
    // For each triangle, the points to add on each of its edges: how far
    // along, and the vertex there.
    let mut splits: HashMap<usize, [Vec<(f64, u32)>; 3]> = HashMap::new();
    for key in &plane_order {
        let triangles = &planes[key];
        if triangles.len() < 2 {
            continue;
        }
        let mut directed: HashMap<([u32; 3], [u32; 3]), ()> = HashMap::new();
        for &t in triangles {
            let [a, b, c] = mesh.triangles[t].corners.map(|k| position_key(at(k)));
            for edge in [(a, b), (b, c), (c, a)] {
                directed.insert(edge, ());
            }
        }
        // Outline edges, and the corners on outlines (one vertex each).
        let mut outline: Vec<(usize, usize)> = Vec::new();
        let mut corners: Vec<u32> = Vec::new();
        let mut seen: HashMap<[u32; 3], ()> = HashMap::new();
        for &t in triangles {
            let ends = mesh.triangles[t].corners;
            for k in 0..3 {
                let (from, to) = (ends[k], ends[(k + 1) % 3]);
                let edge = (position_key(at(from)), position_key(at(to)));
                if directed.contains_key(&(edge.1, edge.0)) {
                    continue;
                }
                outline.push((t, k));
                for vertex in [from, to] {
                    if seen.insert(position_key(at(vertex)), ()).is_none() {
                        corners.push(vertex);
                    }
                }
            }
        }
        if corners.len() < 3 {
            continue;
        }
        // The corners in a grid of about one per cell, to find those near
        // an edge quickly.
        let (low, high) = corners
            .iter()
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(low, high), &v| {
                (low.min(at(v)), high.max(at(v)))
            });
        let cell = ((high - low).max_element() / (corners.len() as f32).sqrt())
            .max(tolerance as f32 * 4.0)
            .max(f32::MIN_POSITIVE);
        let cell_of = |p: Vec3| ((p - low) / cell).floor().as_ivec3();
        let mut cells: HashMap<IVec3, Vec<u32>> = HashMap::new();
        for &vertex in &corners {
            cells.entry(cell_of(at(vertex))).or_default().push(vertex);
        }
        let room = Vec3::splat(tolerance as f32);
        let mut near: Vec<u32> = Vec::new();
        for (t, k) in outline {
            let ends = mesh.triangles[t].corners;
            let (a, b) = (at(ends[k]), at(ends[(k + 1) % 3]));
            let (from, to) = (cell_of(a.min(b) - room), cell_of(a.max(b) + room));
            let span = (to - from + IVec3::ONE).as_i64vec3();
            near.clear();
            if span.x * span.y * span.z > 4096 {
                near.extend_from_slice(&corners);
            } else {
                for x in from.x..=to.x {
                    for y in from.y..=to.y {
                        for z in from.z..=to.z {
                            if let Some(list) = cells.get(&IVec3::new(x, y, z)) {
                                near.extend_from_slice(list);
                            }
                        }
                    }
                }
            }
            let (a, b) = (a.as_dvec3(), b.as_dvec3());
            let along = b - a;
            let length = along.length_squared();
            if length <= 0.0 {
                continue;
            }
            let margin = tolerance / length.sqrt();
            for &vertex in &near {
                let p = at(vertex).as_dvec3();
                if p == a || p == b {
                    continue;
                }
                let fraction = (p - a).dot(along) / length;
                if fraction <= margin || fraction >= 1.0 - margin {
                    continue;
                }
                if (a + along * fraction - p).length() > tolerance {
                    continue;
                }
                splits.entry(t).or_default()[k].push((fraction, vertex));
            }
        }
    }
    if splits.is_empty() {
        return None;
    }

    let mut split = mesh.clone();
    let mut keep = vec![true; mesh.triangles.len()];
    let mut order: Vec<usize> = splits.keys().copied().collect();
    order.sort_unstable();
    for t in order {
        let mut on_edges = splits.remove(&t).unwrap_or_default();
        let triangle = mesh.triangles[t];
        let ends = triangle.corners;
        // The polygon: each corner, then the points along its edge in order.
        let mut ring: Vec<u32> = Vec::new();
        for (k, points) in on_edges.iter_mut().enumerate() {
            points.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
            points.dedup_by(|x, y| position_key(at(x.1)) == position_key(at(y.1)));
            ring.push(ends[k]);
            let (from, to) = (ends[k] as usize, ends[(k + 1) % 3] as usize);
            for &(fraction, vertex) in points.iter() {
                let uv = mesh.uvs[from].lerp(mesh.uvs[to], fraction as f32);
                ring.push(split.push_vertex(at(vertex), mesh.normals[from], uv));
            }
        }
        // Filled flat on its plane.
        let normal = mesh.normals[ends[0] as usize].as_dvec3();
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let flat: Vec<DVec2> = ring
            .iter()
            .map(|&vertex| {
                let p = split.positions[vertex as usize].as_dvec3();
                DVec2::new(p.dot(u), p.dot(v))
            })
            .collect();
        let local: Vec<u32> = (0..ring.len() as u32).collect();
        let filled = crate::fill::fill(&flat, vec![local.clone()]);
        if !crate::fill::has_outline(&filled, &[local]) {
            continue;
        }
        keep[t] = false;
        for [x, y, z] in filled {
            split.triangles.push(Triangle {
                corners: [ring[x as usize], ring[y as usize], ring[z as usize]],
                surface: triangle.surface,
            });
        }
    }
    let mut k = 0;
    split.triangles.retain(|_| {
        k += 1;
        k > keep.len() || keep[k - 1]
    });
    Some(compact(&split))
}

/// What's left of a triangle as faces are cut out of it: convex polygons
/// of points on its plane.
type Pieces = Vec<Vec<DVec2>>;

/// Marks each triangle pressed flat against other solid parts' faces (in
/// one plane, facing it) that cover it all between them: a brick's bottom
/// across two bricks below, or a brick's top under a row laid across it.
/// One only partly covered (a plate's top under a brick) is kept whole:
/// cutting the covered part out would leave more triangles, not fewer.
fn covered(mesh: &Mesh, part_of: &[u32], solid: &dyn Fn(u32) -> bool, tolerance: f64) -> Vec<bool> {
    // Triangles by the way their plane faces, each with its plane's
    // distance from the origin and which way it faces.
    let mut by_normal: HashMap<[i64; 3], Vec<(f64, usize, bool)>> = HashMap::new();
    let mut normal_order: Vec<[i64; 3]> = Vec::new();
    let corners_of = |mesh: &Mesh, t: usize| {
        mesh.triangles[t]
            .corners
            .map(|c| mesh.positions[c as usize].as_dvec3())
    };
    for t in 0..mesh.triangles.len() {
        let [a, b, c] = corners_of(mesh, t);
        let Some(normal) = (b - a).cross(c - a).try_normalize() else {
            continue;
        };
        // One way round for both sides of a plane.
        let first = [normal.x, normal.y, normal.z]
            .into_iter()
            .find(|n| n.abs() > 1e-9)
            .unwrap_or(1.0);
        let (canonical, forward) = if first > 0.0 {
            (normal, true)
        } else {
            (-normal, false)
        };
        let key = (canonical * 1e5).round().as_i64vec3().to_array();
        by_normal
            .entry(key)
            .or_insert_with(|| {
                normal_order.push(key);
                Vec::new()
            })
            .push((canonical.dot(a), t, forward));
    }
    // Planes: runs of those whose distances are within `tolerance` of the
    // one before, so no rounding splits a plane in two.
    // Each plane's way round, and its triangles with which way each faces.
    type Plane = ([i64; 3], Vec<(usize, bool)>);
    let mut planes: Vec<Plane> = Vec::new();
    for key in normal_order {
        let mut list = by_normal.remove(&key).unwrap_or_default();
        list.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let mut last = f64::NEG_INFINITY;
        for (offset, t, forward) in list {
            if offset - last > tolerance || planes.is_empty() {
                planes.push((key, Vec::new()));
            }
            last = offset;
            planes.last_mut().expect("just pushed").1.push((t, forward));
        }
    }

    let mut gone = vec![false; mesh.triangles.len()];
    for (key, group) in &planes {
        if group.iter().all(|&(_, f)| f) || group.iter().all(|&(_, f)| !f) {
            continue;
        }
        // Flat on the plane: u and v across it.
        let normal = DVec3::from_array(key.map(|n| n as f64)).normalize_or_zero();
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let flat = |p: DVec3| DVec2::new(p.dot(u), p.dot(v));
        let flat_of = |t: usize| corners_of(mesh, t).map(flat);
        // The solid parts' triangles in a grid of cells on the plane (about
        // one each), so each triangle only meets those near it.
        let cutters: Vec<usize> = (0..group.len())
            .filter(|&k| solid(part_of[group[k].0]))
            .collect();
        if cutters.is_empty() {
            continue;
        }
        let boxes: Vec<(DVec2, DVec2)> = group.iter().map(|&(t, _)| bounds2(&flat_of(t))).collect();
        let (low, high) = cutters
            .iter()
            .fold((DVec2::INFINITY, DVec2::NEG_INFINITY), |(low, high), &k| {
                (low.min(boxes[k].0), high.max(boxes[k].1))
            });
        let cell = ((high - low).max_element() / (cutters.len() as f64).sqrt())
            .max(tolerance * 16.0)
            .max(f64::MIN_POSITIVE);
        let cell_of = |p: DVec2| ((p - low) / cell).floor().as_i64vec2();
        let mut cells: HashMap<glam::I64Vec2, Vec<usize>> = HashMap::new();
        let mut everywhere: Vec<usize> = Vec::new();
        for &k in &cutters {
            let (from, to) = (cell_of(boxes[k].0), cell_of(boxes[k].1));
            let span = to - from + glam::I64Vec2::ONE;
            if span.x * span.y > 256 {
                everywhere.push(k);
                continue;
            }
            for x in from.x..=to.x {
                for y in from.y..=to.y {
                    cells.entry(glam::I64Vec2::new(x, y)).or_default().push(k);
                }
            }
        }
        let mut near: Vec<usize> = Vec::new();
        for &(t, forward) in group {
            let triangle = flat_of(t);
            let (low, high) = bounds2(&triangle);
            near.clear();
            near.extend_from_slice(&everywhere);
            let (from, to) = (cell_of(low), cell_of(high));
            let span = to - from + glam::I64Vec2::ONE;
            if span.x * span.y > 4096 {
                near.extend_from_slice(&cutters);
            } else {
                for x in from.x..=to.x {
                    for y in from.y..=to.y {
                        if let Some(list) = cells.get(&glam::I64Vec2::new(x, y)) {
                            near.extend_from_slice(list);
                        }
                    }
                }
            }
            near.sort_unstable();
            near.dedup();
            let mut pieces: Pieces = vec![triangle.to_vec()];
            for &k in &near {
                let (other, other_forward) = group[k];
                if other_forward == forward || part_of[other] == part_of[t] {
                    continue;
                }
                let mut cutter = flat_of(other);
                let (other_low, other_high) = boxes[k];
                if other_low.cmpgt(high).any() || other_high.cmplt(low).any() {
                    continue;
                }
                if orient(cutter[0], cutter[1], cutter[2]) < 0.0 {
                    cutter.swap(1, 2);
                }
                pieces = pieces
                    .into_iter()
                    .flat_map(|piece| subtract(&piece, cutter, tolerance))
                    .collect();
                if pieces.is_empty() {
                    break;
                }
            }
            let left: f64 = pieces.iter().map(|piece| area(piece)).sum();
            gone[t] = left <= tolerance * tolerance;
        }
    }
    gone
}

/// The smallest and largest corner of the box around `points`.
fn bounds2(points: &[DVec2]) -> (DVec2, DVec2) {
    points
        .iter()
        .fold((DVec2::INFINITY, DVec2::NEG_INFINITY), |(low, high), &p| {
            (low.min(p), high.max(p))
        })
}

/// The area of the convex polygon `points` (counter-clockwise positive).
fn area(points: &[DVec2]) -> f64 {
    let mut twice = 0.0;
    for k in 0..points.len() {
        let (a, b) = (points[k], points[(k + 1) % points.len()]);
        twice += a.perp_dot(b);
    }
    twice.abs() / 2.0
}

/// What's left of the convex polygon `piece` once the counter-clockwise
/// triangle `cutter` is cut out of it, as convex polygons: the part
/// outside each of the cutter's edges in turn.
fn subtract(piece: &[DVec2], cutter: [DVec2; 3], tolerance: f64) -> Pieces {
    let mut out = Vec::new();
    let mut rest = piece.to_vec();
    for k in 0..3 {
        let (a, b) = (cutter[k], cutter[(k + 1) % 3]);
        let (outside, inside) = split_by_line(&rest, a, b, tolerance);
        if outside.len() >= 3 && area(&outside) > tolerance * tolerance {
            out.push(outside);
        }
        rest = inside;
        if rest.len() < 3 || area(&rest) <= tolerance * tolerance {
            // The cutter doesn't reach any further in: what was outside
            // so far is all there is (nothing was cut when that's all).
            return if out.is_empty() && k == 0 {
                vec![piece.to_vec()]
            } else {
                out
            };
        }
    }
    out
}

/// The convex polygon `points` split by the line through `a` and `b`:
/// the part to its right (outside a counter-clockwise shape) and to its
/// left. Points within `tolerance` of the line go to both.
fn split_by_line(points: &[DVec2], a: DVec2, b: DVec2, tolerance: f64) -> (Vec<DVec2>, Vec<DVec2>) {
    let length = (b - a).length();
    let side = |p: DVec2| {
        let d = orient(a, b, p) / length.max(f64::MIN_POSITIVE);
        if d.abs() <= tolerance { 0.0 } else { d }
    };
    let (mut right, mut left) = (Vec::new(), Vec::new());
    for k in 0..points.len() {
        let (p, q) = (points[k], points[(k + 1) % points.len()]);
        let (sp, sq) = (side(p), side(q));
        if sp <= 0.0 {
            right.push(p);
        }
        if sp >= 0.0 {
            left.push(p);
        }
        if (sp < 0.0 && sq > 0.0) || (sp > 0.0 && sq < 0.0) {
            let t = sp / (sp - sq);
            let cross = p + (q - p) * t;
            right.push(cross);
            left.push(cross);
        }
    }
    (right, left)
}
