//! Terrain tiles: several Terrains laid side by side as one landscape.
//!
//! There is no tile class. Two Terrains are neighbours when they have the
//! same `Size`, the same samples along each side and the same
//! `Position.Y`, and their middles are exactly one width (`Size.X`) or one
//! length (`Size.Z`) apart along X or Z (so they share a whole edge, and
//! every sample on it). [`neighbour_placement`] is where a new tile on a
//! side goes; [`neighbours`] and [`free_sides`] read the world.
//!
//! The two tiles of a seam each keep the edge's samples. [`stitch`] makes
//! them equal (the mean of the two), and [`blend_edge`] pins a new tile's
//! edge to an existing tile's, easing back to its own heights over a band
//! of samples, so a tile added beside sculpted ground meets it exactly.
//! Holes and painted water are per cell (from a sample to +X and +Z), and
//! a seam's cells belong to one tile or the other, so only heights and
//! paint (per sample) are shared along an edge.

use axcenity_core::{ClassName, InstanceId, MAX_TERRAIN_LAYERS, SampleRect, TerrainData, World};
use glam::Vec3;

use crate::Placement;
use crate::noise::smoothstep;

/// How many samples [`blend_edge`] eases over by default.
pub const BLEND_BAND: usize = 16;

/// One side of a tile, by the direction across it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    NegX,
    PosX,
    NegZ,
    PosZ,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::NegZ, Side::PosX, Side::PosZ, Side::NegX];

    /// The side facing it, of the tile across.
    pub fn opposite(self) -> Side {
        match self {
            Side::NegX => Side::PosX,
            Side::PosX => Side::NegX,
            Side::NegZ => Side::PosZ,
            Side::PosZ => Side::NegZ,
        }
    }

    /// One step across it, along X and Z (-1, 0 or 1).
    pub fn step(self) -> (f32, f32) {
        match self {
            Side::NegX => (-1.0, 0.0),
            Side::PosX => (1.0, 0.0),
            Side::NegZ => (0.0, -1.0),
            Side::PosZ => (0.0, 1.0),
        }
    }

    /// Its name for people: the axis and its direction.
    pub fn label(self) -> &'static str {
        match self {
            Side::NegX => "−X",
            Side::PosX => "+X",
            Side::NegZ => "−Z",
            Side::PosZ => "+Z",
        }
    }

    /// The sample `depth` samples in from this side, `along` it (along Z
    /// for an X side, along X for a Z side), on a `size` grid.
    fn sample(self, size: (usize, usize), along: usize, depth: usize) -> (usize, usize) {
        let (size_x, size_z) = size;
        match self {
            Side::NegX => (depth, along),
            Side::PosX => (size_x - 1 - depth, along),
            Side::NegZ => (along, depth),
            Side::PosZ => (along, size_z - 1 - depth),
        }
    }

    /// How many samples lie along this side, and how deep the grid is from it.
    fn extent(self, size: (usize, usize)) -> (usize, usize) {
        match self {
            Side::NegX | Side::PosX => (size.1, size.0),
            Side::NegZ | Side::PosZ => (size.0, size.1),
        }
    }

    /// This side's row (or column) of samples.
    pub fn edge_rect(self, size: (usize, usize)) -> SampleRect {
        let (size_x, size_z) = size;
        match self {
            Side::NegX => SampleRect::new(0, 0, 1, size_z),
            Side::PosX => SampleRect::new(size_x - 1, 0, size_x, size_z),
            Side::NegZ => SampleRect::new(0, 0, size_x, 1),
            Side::PosZ => SampleRect::new(0, size_z - 1, size_x, size_z),
        }
    }

    /// The samples within `depth` of this side.
    pub fn band_rect(self, size: (usize, usize), depth: usize) -> SampleRect {
        let (size_x, size_z) = size;
        let depth = depth.max(1);
        match self {
            Side::NegX => SampleRect::new(0, 0, depth.min(size_x), size_z),
            Side::PosX => SampleRect::new(size_x.saturating_sub(depth), 0, size_x, size_z),
            Side::NegZ => SampleRect::new(0, 0, size_x, depth.min(size_z)),
            Side::PosZ => SampleRect::new(0, size_z.saturating_sub(depth), size_x, size_z),
        }
    }
}

/// Where a tile beside `placement` on `side` goes: the same size, one
/// width or length along, so the two share that edge.
pub fn neighbour_placement(placement: Placement, side: Side) -> Placement {
    let (dx, dz) = side.step();
    Placement {
        position: placement.position + Vec3::new(dx * placement.size.x, 0.0, dz * placement.size.z),
        size: placement.size,
    }
}

/// Whether two numbers are the same, give or take float rounding of a
/// value about `scale` big.
fn same(a: f32, b: f32, scale: f32) -> bool {
    (a - b).abs() <= 1e-4 * scale.abs().max(1.0)
}

/// The side of tile `a` that tile `b` sits on, if they're neighbours (see
/// the module docs). `samples` are each one's samples along X and Z.
pub fn side_between(
    a: Placement,
    a_samples: (usize, usize),
    b: Placement,
    b_samples: (usize, usize),
) -> Option<Side> {
    let scale = a.size.max_element();
    let same_size = (0..3).all(|axis| same(a.size[axis], b.size[axis], scale));
    if !same_size || a_samples != b_samples || !same(a.position.y, b.position.y, scale) {
        return None;
    }
    Side::ALL.into_iter().find(|&side| {
        let at = neighbour_placement(a, side).position;
        same(at.x, b.position.x, scale) && same(at.z, b.position.z, scale)
    })
}

/// A Terrain's placement and samples, if it has both.
fn tile(world: &World, id: InstanceId) -> Option<(Placement, (usize, usize))> {
    let placement = Placement::of(world.get(id)?)?;
    Some((placement, world.terrain(id)?.size()))
}

/// Every Terrain in the Workspace.
fn terrains(world: &World) -> Vec<InstanceId> {
    world.all_below(world.workspace(), |class| class == ClassName::Terrain)
}

/// The Workspace's Terrains that would be neighbours of a tile at
/// `placement` with `samples`, with the side of it each is on.
pub fn neighbours_of(
    world: &World,
    placement: Placement,
    samples: (usize, usize),
    except: Option<InstanceId>,
) -> Vec<(Side, InstanceId)> {
    terrains(world)
        .into_iter()
        .filter(|&other| Some(other) != except)
        .filter_map(|other| {
            let (theirs, their_samples) = tile(world, other)?;
            let side = side_between(placement, samples, theirs, their_samples)?;
            Some((side, other))
        })
        .collect()
}

/// The Terrain `id`'s neighbours, with the side of it each is on.
pub fn neighbours(world: &World, id: InstanceId) -> Vec<(Side, InstanceId)> {
    let Some((placement, samples)) = tile(world, id) else {
        return Vec::new();
    };
    neighbours_of(world, placement, samples, Some(id))
}

/// The Terrain `id`'s neighbour on `side`, if it has one.
pub fn neighbour_on(world: &World, id: InstanceId, side: Side) -> Option<InstanceId> {
    neighbours(world, id)
        .into_iter()
        .find(|&(on, _)| on == side)
        .map(|(_, other)| other)
}

/// Every tile of the landscape `id` is in: it and its neighbours, their
/// neighbours and so on, in the order they're reached.
pub fn connected(world: &World, id: InstanceId) -> Vec<InstanceId> {
    let mut found = vec![id];
    let mut next = 0;
    while next < found.len() {
        for (_, other) in neighbours(world, found[next]) {
            if !found.contains(&other) {
                found.push(other);
            }
        }
        next += 1;
    }
    found
}

/// The sides of the Terrain `id` with nothing across them: where a new
/// tile can go. A side is taken by a neighbour or by any Terrain whose
/// footprint overlaps where the new tile would be.
pub fn free_sides(world: &World, id: InstanceId) -> Vec<Side> {
    let Some((placement, _)) = tile(world, id) else {
        return Vec::new();
    };
    let others: Vec<Placement> = terrains(world)
        .into_iter()
        .filter(|&other| other != id)
        .filter_map(|other| Placement::of(world.get(other)?))
        .collect();
    Side::ALL
        .into_iter()
        .filter(|&side| {
            let wanted = neighbour_placement(placement, side);
            !others.iter().any(|other| overlaps(wanted, *other))
        })
        .collect()
}

/// Whether two footprints overlap by more than float rounding (sharing an
/// edge isn't overlapping).
fn overlaps(a: Placement, b: Placement) -> bool {
    let margin = 1e-3 * a.size.max_element().max(1.0);
    let apart_x = (a.position.x - b.position.x).abs() + margin >= (a.size.x + b.size.x) / 2.0;
    let apart_z = (a.position.z - b.position.z).abs() + margin >= (a.size.z + b.size.z) / 2.0;
    !apart_x && !apart_z
}

/// Makes the shared edge of neighbours `a` and `b` (on `a`'s `side`) the
/// same on both: each sample's height and paint becomes the mean of the
/// two. Returns the rects changed in `a` and in `b`, or `None` if their
/// sample counts differ.
pub fn stitch(
    a: &mut TerrainData,
    b: &mut TerrainData,
    side: Side,
) -> Option<(SampleRect, SampleRect)> {
    if a.size() != b.size() {
        return None;
    }
    let size = a.size();
    let (along, _) = side.extent(size);
    let (rect_a, rect_b) = (side.edge_rect(size), side.opposite().edge_rect(size));
    let pairs: Vec<(usize, usize)> = (0..along)
        .map(|i| {
            let (ax, az) = side.sample(size, i, 0);
            let (bx, bz) = side.opposite().sample(size, i, 0);
            (a.index(ax, az), b.index(bx, bz))
        })
        .collect();
    let mean_heights: Vec<f32> = pairs
        .iter()
        .map(|&(ia, ib)| (a.heights()[ia] + b.heights()[ib]) / 2.0)
        .collect();
    let mean_paint: Vec<[u8; MAX_TERRAIN_LAYERS]> = pairs
        .iter()
        .map(|&(ia, ib)| {
            let (pa, pb) = (a.paint()[ia], b.paint()[ib]);
            std::array::from_fn(|layer| ((pa[layer] as u16 + pb[layer] as u16).div_ceil(2)) as u8)
        })
        .collect();
    for (data, rect, pick) in [(&mut *a, rect_a, 0), (&mut *b, rect_b, 1)] {
        let (heights, paint) = data.both_mut_in(rect);
        for (k, pair) in pairs.iter().enumerate() {
            let index = if pick == 0 { pair.0 } else { pair.1 };
            heights[index] = mean_heights[k];
            paint[index] = mean_paint[k];
        }
    }
    Some((rect_a, rect_b))
}

/// Pins the edge of `new` that meets `from` (`new` sits on `from`'s
/// `side`) to `from`'s edge: the edge's heights and paint become
/// `from`'s, and the difference in height eases away over `band`
/// samples, so past the band `new` keeps its own ground. Returns the rect
/// of `new` changed, or `None` if their sample counts differ.
pub fn blend_edge(
    new: &mut TerrainData,
    from: &TerrainData,
    side: Side,
    band: usize,
) -> Option<SampleRect> {
    if new.size() != from.size() {
        return None;
    }
    let size = new.size();
    let facing = side.opposite();
    let (along, deep) = facing.extent(size);
    let band = band.clamp(1, deep);
    let rect = facing.band_rect(size, band);
    let edge: Vec<(f32, [u8; MAX_TERRAIN_LAYERS])> = (0..along)
        .map(|i| {
            let (x, z) = side.sample(size, i, 0);
            let index = from.index(x, z);
            (from.heights()[index], from.paint()[index])
        })
        .collect();
    let indices: Vec<Vec<usize>> = (0..along)
        .map(|i| {
            (0..band)
                .map(|depth| {
                    let (x, z) = facing.sample(size, i, depth);
                    new.index(x, z)
                })
                .collect()
        })
        .collect();
    let (heights, paint) = new.both_mut_in(rect);
    for (i, row) in indices.iter().enumerate() {
        let (pinned, pinned_paint) = edge[i];
        let difference = pinned - heights[row[0]];
        for (depth, &index) in row.iter().enumerate() {
            // 1 at the edge, easing to 0 at the band's end.
            let keep = 1.0 - smoothstep(0.0, band as f32, depth as f32);
            heights[index] = (heights[index] + difference * keep).clamp(0.0, 1.0);
        }
        heights[row[0]] = pinned;
        paint[row[0]] = pinned_paint;
    }
    Some(rect)
}

/// The mean height of `data`'s `side` edge, 0 to 1.
pub fn edge_height(data: &TerrainData, side: Side) -> f32 {
    let size = data.size();
    let (along, _) = side.extent(size);
    let total: f32 = (0..along)
        .map(|i| {
            let (x, z) = side.sample(size, i, 0);
            data.height(x as isize, z as isize)
        })
        .sum();
    total / along as f32
}

/// The heights along `data`'s `side` edge, in order (for tests and checks).
pub fn edge_heights(data: &TerrainData, side: Side) -> Vec<f32> {
    let size = data.size();
    let (along, _) = side.extent(size);
    (0..along)
        .map(|i| {
            let (x, z) = side.sample(size, i, 0);
            data.height(x as isize, z as isize)
        })
        .collect()
}

#[cfg(test)]
#[path = "tiles_tests.rs"]
mod tests;
