//! Terrain water (LE4, `docs/terrain.md`), drawn as the renderer's water
//! (`render::Water`, `docs/rendering/water.md`), two ways:
//!
//! - **Flooding**: a Terrain with `Water` on is flooded up to its
//!   `WaterLevel` (world Y) over every chunk whose lowest ground is below
//!   that level, so seas, lakes and rivers fill the hollows the ground
//!   leaves under it, with foam where they meet it. Each chunk's water is
//!   the top of a box as wide as the chunk, cut into the water grid.
//! - **Painted water** (the Water brush, `TerrainData::water`): each chunk
//!   with painted cells gets a mesh of just those cells, each at its own
//!   level and cut as finely as the water grid, leaving out cells whose
//!   ground is above their water and, with flooding on, cells flooded
//!   anyway. It's built again only when the chunk's water or heights, or
//!   the flood level, change.
//!
//! Both have the Terrain's look (`WaterColor`, the waves), and the waves
//! follow world positions, so chunks, and painted cells, meet seamlessly.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use axcenity_core::{Instance, InstanceId, TerrainData, Value};
use axcenity_render::glam::{Mat4, Quat, Vec3};
use axcenity_render::{DrawItem, Material, MeshSource, Shape, WATER_GRID, Water};
use axcenity_terrain::Placement;

/// The water a Terrain draws: its look, and the level it floods to while
/// its `Water` is on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerrainWater {
    /// World Y of the flood's surface at rest, or `None` while `Water` is off.
    pub(crate) level: Option<f32>,
    pub(crate) water: Water,
}

impl TerrainWater {
    pub(crate) fn of(terrain: &Instance) -> Self {
        let number = |name: &str, otherwise: f32| {
            terrain
                .property(name)
                .and_then(Value::as_number)
                .map(|value| value as f32)
                .filter(|value| value.is_finite())
                .unwrap_or(otherwise)
        };
        let defaults = Water::default();
        let deep_color = terrain
            .property("WaterColor")
            .and_then(Value::as_color3)
            .map_or(defaults.deep_color, |color| [color.r, color.g, color.b]);
        let flooded = terrain.property("Water").and_then(Value::as_bool) == Some(true);
        Self {
            level: flooded.then(|| number("WaterLevel", 0.0)),
            water: Water {
                deep_color,
                clarity: number("WaterClarity", defaults.clarity).max(0.01),
                wave_height: number("WaveHeight", defaults.wave_height).max(0.0),
                wave_length: number("WaveLength", defaults.wave_length).max(0.5),
                wave_speed: number("WaveSpeed", defaults.wave_speed).max(0.0),
                wave_direction: number("WaveDirection", defaults.wave_direction),
                wave_variation: number("WaveVariation", defaults.wave_variation).clamp(0.0, 1.0),
                ..defaults
            },
        }
    }
}

/// What a terrain's water keeps between frames: each chunk's lowest ground
/// (world Y), with the heights' version it was worked out from, and each
/// chunk's painted water mesh, with what it was built from.
#[derive(Default)]
pub(crate) struct WaterCache {
    lowest: HashMap<(usize, usize), (u64, f32)>,
    painted: HashMap<(usize, usize), BuiltWater>,
}

/// A chunk's painted water mesh (`None` when nothing in it shows), with
/// the key of what it was built from.
type BuiltWater = (u64, Option<Arc<PaintedWater>>);

impl WaterCache {
    fn lowest(
        &mut self,
        data: &TerrainData,
        placement: &Placement,
        (cx, cz): (usize, usize),
    ) -> f32 {
        let version = data.chunk_height_version(cx, cz);
        if let Some(&(built, lowest)) = self.lowest.get(&(cx, cz))
            && built == version
        {
            return lowest;
        }
        let rect = data.chunk_rect(cx, cz);
        let mut lowest = f32::INFINITY;
        for z in rect.z0..rect.z1 {
            for x in rect.x0..rect.x1 {
                lowest = lowest.min(data.height(x as isize, z as isize));
            }
        }
        let lowest = placement.world_y(lowest);
        self.lowest.insert((cx, cz), (version, lowest));
        lowest
    }

    /// Chunk (`cx`, `cz`)'s painted water mesh, built again only when what
    /// it's made from changed; `None` when nothing in it shows.
    fn painted(
        &mut self,
        id: InstanceId,
        data: &TerrainData,
        placement: &Placement,
        flood: Option<f32>,
        (cx, cz): (usize, usize),
    ) -> Option<Arc<PaintedWater>> {
        let mut hasher = DefaultHasher::new();
        (
            data.uid(),
            data.chunk_water_version(cx, cz),
            data.chunk_height_version(cx, cz),
            flood.map(f32::to_bits),
            placement.position.to_array().map(f32::to_bits),
            placement.size.to_array().map(f32::to_bits),
        )
            .hash(&mut hasher);
        let built = hasher.finish();
        if let Some((key, mesh)) = self.painted.get(&(cx, cz))
            && *key == built
        {
            return mesh.clone();
        }
        let mesh = PaintedWater::build(id, data, placement, flood, (cx, cz), built).map(Arc::new);
        self.painted.insert((cx, cz), (built, mesh.clone()));
        mesh
    }
}

/// Adds the terrain's water to `items`: the flood over each chunk whose
/// ground dips below its level, and the painted water that shows.
pub(crate) fn draw(
    id: InstanceId,
    water: &TerrainWater,
    data: &TerrainData,
    placement: &Placement,
    cache: &mut WaterCache,
    items: &mut Vec<DrawItem>,
) {
    let (chunks_x, chunks_z) = data.chunk_counts();
    let painted = data.has_water();
    if water.level.is_none() && !painted {
        return;
    }
    for cz in 0..chunks_z {
        for cx in 0..chunks_x {
            let (first, width, depth) = chunk_footprint(data, placement, (cx, cz));
            let middle = |y: f32| Vec3::new(first.x + width * 0.5, y, first.z + depth * 0.5);
            let scale = Vec3::new(width, 1.0, depth);
            if let Some(level) = water.level
                && cache.lowest(data, placement, (cx, cz)) < level
            {
                let transform = Mat4::from_scale_rotation_translation(
                    scale,
                    Quat::IDENTITY,
                    middle(level - 0.5),
                );
                items.push(water_item(
                    water,
                    Shape::Mesh(axcenity_render::water_surface()),
                    transform,
                ));
            }
            if painted
                && data.chunk_has_water(cx, cz)
                && let Some(mesh) = cache.painted(id, data, placement, water.level, (cx, cz))
            {
                // The mesh is the chunk's footprint across (-0.5 to 0.5),
                // at world heights: the same scale as the flood's grid, so
                // the waves fade alike.
                let transform =
                    Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, middle(0.0));
                items.push(water_item(water, Shape::Mesh(mesh), transform));
            }
        }
    }
}

/// Chunk (`cx`, `cz`)'s first sample (world) and its width and depth in
/// studs: its rect ends one past its last sample, which it shares with the
/// next chunk.
fn chunk_footprint(
    data: &TerrainData,
    placement: &Placement,
    (cx, cz): (usize, usize),
) -> (Vec3, f32, f32) {
    let rect = data.chunk_rect(cx, cz);
    let cell = placement.cell_size(data);
    let x1 = (rect.x1 - 1).max(rect.x0 + 1);
    let z1 = (rect.z1 - 1).max(rect.z0 + 1);
    let first = placement.sample_point(data, rect.x0, rect.z0);
    (
        first,
        (x1 - rect.x0) as f32 * cell.x,
        (z1 - rect.z0) as f32 * cell.y,
    )
}

/// One chunk's water.
fn water_item(water: &TerrainWater, shape: Shape, transform: Mat4) -> DrawItem {
    DrawItem {
        shape,
        transform,
        material: Material {
            water: Some(water.water),
            ..Material::default()
        },
        uv_tile: 0.0,
        casts_shadow: false,
        id: 0,
        cull_distance: 0.0,
        skin: None,
        sway: None,
        decal_role: Default::default(),
    }
}

/// A chunk's painted water: a square of the water grid's fineness for each
/// cell that shows, at its level. X and Z go from -0.5 to 0.5 across the
/// chunk; Y is the world's.
pub(crate) struct PaintedWater {
    key: u64,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl PaintedWater {
    fn build(
        id: InstanceId,
        data: &TerrainData,
        placement: &Placement,
        flood: Option<f32>,
        (cx, cz): (usize, usize),
        built: u64,
    ) -> Option<Self> {
        let rect = data.chunk_rect(cx, cz);
        let cells_x = (rect.x1 - 1).max(rect.x0 + 1) - rect.x0;
        let cells_z = (rect.z1 - 1).max(rect.z0 + 1) - rect.z0;
        // Squares a cell is cut into each way, so a chunk is as fine as the
        // water grid.
        let cuts = (WATER_GRID as usize / cells_x.max(cells_z)).max(1);
        let ground = |x: usize, z: usize| placement.world_y(data.height(x as isize, z as isize));
        let mut mesh = PaintedWater {
            key: 0,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: Vec::new(),
        };
        for z in rect.z0..rect.z0 + cells_z {
            for x in rect.x0..rect.x0 + cells_x {
                let Some(level) = data.water_at(x, z) else {
                    continue;
                };
                let level = placement.world_y(level);
                // Flooded anyway, or the ground is above it all round.
                if flood.is_some_and(|flood| level <= flood + 0.01) {
                    continue;
                }
                let corners = [
                    ground(x, z),
                    ground(x + 1, z),
                    ground(x, z + 1),
                    ground(x + 1, z + 1),
                ];
                if corners.iter().all(|&y| y >= level) {
                    continue;
                }
                let first = mesh.positions.len() as u32;
                let side = cuts + 1;
                for j in 0..=cuts {
                    for i in 0..=cuts {
                        let u = ((x - rect.x0) as f32 + i as f32 / cuts as f32) / cells_x as f32;
                        let v = ((z - rect.z0) as f32 + j as f32 / cuts as f32) / cells_z as f32;
                        mesh.positions.push([u - 0.5, level, v - 0.5]);
                        mesh.normals.push([0.0, 1.0, 0.0]);
                        mesh.uvs.push([u, v]);
                        mesh.tangents.push([1.0, 0.0, 0.0, -1.0]);
                    }
                }
                for j in 0..cuts {
                    for i in 0..cuts {
                        let at = |i: usize, j: usize| first + (j * side + i) as u32;
                        let (a, b, c, d) = (at(i, j), at(i, j + 1), at(i + 1, j), at(i + 1, j + 1));
                        // Counter-clockwise seen from above.
                        mesh.indices.extend([a, b, c, c, b, d]);
                    }
                }
            }
        }
        if mesh.indices.is_empty() {
            return None;
        }
        let mut hasher = DefaultHasher::new();
        ("terrain painted water", id, cx, cz, built).hash(&mut hasher);
        mesh.key = hasher.finish();
        Some(mesh)
    }
}

impl MeshSource for PaintedWater {
    fn key(&self) -> u64 {
        self.key
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &self.uvs
    }
    fn tangents(&self) -> &[[f32; 4]] {
        &self.tangents
    }
    fn indices(&self) -> &[u32] {
        &self.indices
    }
}
