//! Terrains as draw items: one per chunk, with its level of detail picked
//! by its distance from the camera, and its blended layers.
//!
//! Building chunk meshes and weights takes time, so a [`TerrainCache`]
//! (owned by the host, like [`Particles`](crate::Particles)) keeps them and
//! only rebuilds chunks whose heights or paint changed (each chunk counts
//! its edits, see `core::TerrainData`), or all of them when the layers'
//! rules change. A changed chunk mesh keeps its key and gets a new
//! version, so the renderer rewrites its GPU copy in place.
//!
//! Each chunk's level is picked first, then its mesh is built to meet its
//! neighbours' levels (`terrain::mesh::side_lods`), so the surface has no
//! cracks wherever two levels meet. A mesh takes well under a millisecond,
//! so it's always built in the frame that draws it: a chunk is never left
//! out, or drawn at a level its neighbours don't expect, while it waits.
//!
//! With world streaming on (Phase 80), chunks out of range are dropped
//! and chunks that come back in range have their weights (the slow part)
//! made on a background thread, and are drawn once those are ready.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use axcenity_assets::Assets;
use axcenity_core::{
    Instance, InstanceId, MAX_TERRAIN_LAYERS, TerrainData, TextureChannel, Value, World,
};
use axcenity_render::glam::{Mat4, Vec3};
use axcenity_render::{
    Camera, Channel, ChannelMap, DrawItem, ItemChunk, Material, MeshSource, NormalMap, Shape,
    TerrainLayer, TerrainLayers, TerrainSurface, TerrainView, TextureSource,
};
use axcenity_terrain::materials::{self, MaterialRules};
use axcenity_terrain::mesh::{self, ChunkMesh, LOD_LEVELS};
use axcenity_terrain::{CHUNK_CELLS, Placement};

use crate::draw_list::image_texture;
use crate::streaming::Streaming;

/// Chunks nearer the camera than this many studs are drawn at full detail;
/// each doubling of the distance past it halves the detail.
pub const DEFAULT_DETAIL_DISTANCE: f32 = 160.0;

/// Colors for the Weights view, one per layer.
const DEBUG_COLORS: [[f32; 3]; MAX_TERRAIN_LAYERS] = [
    [0.35, 0.75, 0.3],
    [0.6, 0.6, 0.62],
    [0.95, 0.95, 1.0],
    [0.85, 0.75, 0.45],
    [0.55, 0.38, 0.22],
    [0.25, 0.5, 0.95],
    [0.9, 0.35, 0.3],
    [0.7, 0.4, 0.85],
];

/// What's kept between frames for drawing terrains. See the module docs.
pub struct TerrainCache {
    terrains: HashMap<InstanceId, Entry>,
    /// Landscapes drawn in place of terrains' own (the editor's previews).
    /// Each with the Size it's drawn at, if not the terrain's own.
    overrides: HashMap<InstanceId, (Arc<TerrainData>, Option<Vec3>)>,
    /// How terrains are shown.
    pub view: TerrainView,
    pub detail_distance: f32,
    /// What's in range of the camera (Phase 80). The host calls its
    /// `step` each frame; drawing leaves out what's hidden.
    pub streaming: Streaming,
    /// Makes chunks' weights in the background while streaming, started
    /// the first time one is needed.
    chunk_worker: Option<ChunkWorker>,
    /// Chunks whose weights are being made, by terrain and chunk.
    pending: HashSet<ChunkId>,
    /// MeshParts' meshes, materials and skins, kept between frames (I8).
    pub(crate) meshes: crate::mesh_cache::MeshCache,
    /// Instances' draw items, kept between frames (M4).
    pub(crate) draw_cache: crate::retained::DrawCache,
    /// Moving parts drawn between physics steps this frame (roadmap G3).
    pub(crate) blends: crate::blend::Blends,
}

impl Default for TerrainCache {
    fn default() -> Self {
        Self {
            terrains: HashMap::new(),
            overrides: HashMap::new(),
            view: TerrainView::Shaded,
            detail_distance: DEFAULT_DETAIL_DISTANCE,
            streaming: Streaming::default(),
            chunk_worker: None,
            pending: HashSet::new(),
            meshes: Default::default(),
            draw_cache: crate::retained::DrawCache::new(crate::retained::on_by_default()),
            blends: crate::blend::Blends::default(),
        }
    }
}

/// Which chunk's weights are being made.
type ChunkId = (InstanceId, usize, usize);

/// A chunk's weights to make away from the frame.
struct ChunkJob {
    id: InstanceId,
    uid: u64,
    cx: usize,
    cz: usize,
    weights_key: WeightsKey,
    data: Arc<TerrainData>,
    placement: Placement,
    rules: MaterialRules,
}

/// What a [`ChunkJob`] made.
struct ChunkDone {
    id: InstanceId,
    uid: u64,
    cx: usize,
    cz: usize,
    weights_key: WeightsKey,
    weights: [Arc<dyn TextureSource>; 2],
}

/// A background thread for chunks, which stops when this is dropped.
struct ChunkWorker {
    jobs: Sender<ChunkJob>,
    done: Receiver<ChunkDone>,
}

impl ChunkWorker {
    fn start() -> ChunkWorker {
        let (jobs, inbox) = mpsc::channel::<ChunkJob>();
        let (outbox, done) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(job) = inbox.recv() {
                let weights = weight_textures(
                    &job.data,
                    &job.placement,
                    &job.rules,
                    (job.cx, job.cz),
                    job.id,
                    job.weights_key,
                );
                let result = ChunkDone {
                    id: job.id,
                    uid: job.uid,
                    cx: job.cx,
                    cz: job.cz,
                    weights_key: job.weights_key,
                    weights,
                };
                if outbox.send(result).is_err() {
                    return;
                }
            }
        });
        ChunkWorker { jobs, done }
    }
}

/// One terrain's chunks, for the data and size they were built from.
struct Entry {
    uid: u64,
    samples: (usize, usize),
    size: Vec3,
    chunks: HashMap<(usize, usize), Chunk>,
    layers: Option<Arc<TerrainLayers>>,
    /// Its painted plants and models, built once.
    foliage: Option<crate::foliage::PaintedFoliage>,
    /// Each chunk's lowest ground and painted water (`terrain_water.rs`).
    water: crate::terrain_water::WaterCache,
}

#[derive(Default)]
struct Chunk {
    /// Each level of detail's mesh, with the heights' version and the
    /// sides' levels it was built for.
    meshes: [Option<BuiltMesh>; LOD_LEVELS as usize],
    /// The weight textures, with the versions and rules they were made from.
    weights: Option<(WeightsKey, [Arc<dyn TextureSource>; 2])>,
}

/// A chunk mesh with the heights' version and the sides' levels it was
/// built for.
type BuiltMesh = (u64, [u32; 4], Arc<ChunkSource>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct WeightsKey {
    heights: u64,
    paint: u64,
    rules: u64,
}

impl TerrainCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The moving parts to draw between physics steps from the next frame
    /// on (roadmap G3), until the host sets others. Hosts set them every
    /// frame from the physics step's.
    pub fn set_blends(&mut self, blends: crate::blend::Blends) {
        self.blends = blends;
    }

    /// One that's thrown away after one frame, with nothing kept: there's
    /// no later frame for retained draw items to help.
    pub(crate) fn for_one_frame() -> Self {
        let mut cache = Self::default();
        cache.set_retained_draw_list(false);
        cache
    }

    /// Whether Instances' draw items are kept between frames: `true` from
    /// [`new`](Self::new) unless the environment variable
    /// `AXCENITY_RETAINED_DRAW_LIST` is `off` or `0`.
    pub fn retained_draw_list(&self) -> bool {
        self.draw_cache.enabled()
    }

    /// Turns keeping Instances' draw items between frames on or off.
    pub fn set_retained_draw_list(&mut self, on: bool) {
        self.draw_cache = crate::retained::DrawCache::new(on);
    }

    /// Draws `data` in place of the Terrain `id`'s own (or its own again
    /// for `None`), at `size` studs if given, without changing the world:
    /// for previews.
    pub fn set_override(
        &mut self,
        id: InstanceId,
        data: Option<Arc<TerrainData>>,
        size: Option<Vec3>,
    ) {
        match data {
            Some(data) => self.overrides.insert(id, (data, size)),
            None => self.overrides.remove(&id),
        };
    }

    pub fn has_override(&self, id: InstanceId) -> bool {
        self.overrides.contains_key(&id)
    }

    /// Draws every terrain's own landscape again.
    pub fn clear_overrides(&mut self) {
        self.overrides.clear();
    }

    /// Forgets everything (another project, or Play and Stop).
    pub fn clear(&mut self) {
        self.terrains.clear();
        self.overrides.clear();
        self.streaming = Streaming::default();
        self.pending.clear();
    }

    /// How many chunks' weights are still being made in the background.
    pub fn pending_chunks(&self) -> usize {
        self.pending.len()
    }

    /// Puts chunks the background thread finished where drawing finds them.
    fn take_finished_chunks(&mut self) {
        let Some(worker) = &self.chunk_worker else {
            return;
        };
        while let Ok(done) = worker.done.try_recv() {
            self.pending.remove(&(done.id, done.cx, done.cz));
            let Some(entry) = self.terrains.get_mut(&done.id) else {
                continue;
            };
            if entry.uid != done.uid {
                continue;
            }
            let chunk = entry.chunks.entry((done.cx, done.cz)).or_default();
            chunk.weights = Some((done.weights_key, done.weights));
        }
    }

    /// The landscape drawn for the Terrain `id`: an override, or its own.
    pub fn data<'a>(&'a self, world: &'a World, id: InstanceId) -> Option<&'a Arc<TerrainData>> {
        self.overrides
            .get(&id)
            .map(|(data, _)| data)
            .or_else(|| world.terrain(id))
    }

    /// Builds the Terrain `id`'s chunks as `camera` would see them, ahead
    /// of the first frame that draws it (such as on a loading screen).
    pub fn prepare(&mut self, world: &World, id: InstanceId, assets: &Assets, camera: &Camera) {
        if let Some(instance) = world.get(id) {
            // On a loading screen the chunks are built right away.
            self.draw(
                world,
                id,
                instance,
                assets,
                camera,
                &mut Vec::new(),
                &mut Vec::new(),
                false,
            );
        }
    }

    /// Adds the chunks of every Terrain in the Workspace to `items`, and
    /// forgets terrains that are gone.
    pub(crate) fn draw_all(
        &mut self,
        world: &World,
        assets: &Assets,
        camera: &Camera,
        items: &mut Vec<DrawItem>,
        chunks: &mut Vec<ItemChunk>,
    ) {
        self.take_finished_chunks();
        let background = self.streaming.settings().is_some();
        let mut seen = HashSet::new();
        for id in world.all_below(world.workspace(), |class| {
            class == axcenity_core::ClassName::Terrain
        }) {
            let Some(instance) = world.get(id) else {
                continue;
            };
            seen.insert(id);
            self.draw(
                world, id, instance, assets, camera, items, chunks, background,
            );
        }
        self.terrains.retain(|id, _| seen.contains(id));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        world: &World,
        id: InstanceId,
        instance: &Instance,
        assets: &Assets,
        camera: &Camera,
        items: &mut Vec<DrawItem>,
        chunks: &mut Vec<ItemChunk>,
        background: bool,
    ) {
        let Some(mut placement) = Placement::of(instance) else {
            return;
        };
        if let Some((_, Some(size))) = self.overrides.get(&id) {
            placement.size = *size;
        }
        let Some(data) = self.data(world, id).cloned() else {
            return;
        };
        let rules = MaterialRules::of(world, id);
        let rules_key = rules.key();
        let layers = layers_of(world, id, &placement, &data, assets, self.view);
        let entry = self.terrains.entry(id).or_insert_with(|| Entry {
            uid: data.uid(),
            samples: data.size(),
            size: placement.size,
            chunks: HashMap::new(),
            layers: None,
            foliage: None,
            water: Default::default(),
        });
        // Another landscape or another size: nothing kept fits it.
        if entry.uid != data.uid() || entry.samples != data.size() || entry.size != placement.size {
            *entry = Entry {
                uid: data.uid(),
                samples: data.size(),
                size: placement.size,
                chunks: HashMap::new(),
                layers: None,
                foliage: None,
                water: Default::default(),
            };
        }
        let layers = match &entry.layers {
            Some(kept) if kept.key == layers.key => kept.clone(),
            _ => {
                let layers = Arc::new(layers);
                entry.layers = Some(layers.clone());
                layers
            }
        };
        let casts_shadow = instance.property("CastShadow").and_then(Value::as_bool) != Some(false);
        let transform = Mat4::from_translation(placement.position);
        let cell = placement.cell_size(&data);
        let (chunks_x, chunks_z) = data.chunk_counts();
        // First each chunk's level (`None` for one out of range), so each
        // chunk's mesh can meet its neighbours' with no cracks.
        let mut lods: Vec<Option<u32>> = vec![None; chunks_x * chunks_z];
        for cz in 0..chunks_z {
            for cx in 0..chunks_x {
                let rect = data.chunk_rect(cx, cz);
                let middle = (rect.x0 + rect.x1 - 1) / 2;
                let middle_z = (rect.z0 + rect.z1 - 1) / 2;
                let center = placement.sample_point(&data, middle, middle_z);
                // The nearest point of the chunk's footprint counts, so a
                // big chunk right under the camera is still detailed.
                let half = Vec3::new(
                    CHUNK_CELLS as f32 * cell.x / 2.0,
                    0.0,
                    CHUNK_CELLS as f32 * cell.y / 2.0,
                );
                // Streaming (Phase 80): out of range, a chunk goes off
                // the CPU and (no longer drawn) the GPU.
                if let Some((keep, build)) = self.streaming.chunk_in_range(center, half) {
                    if !keep {
                        entry.chunks.remove(&(cx, cz));
                        continue;
                    }
                    if !build && !entry.chunks.contains_key(&(cx, cz)) {
                        continue;
                    }
                }
                let offset = (camera.position - center).abs() - half;
                let distance = offset.max(Vec3::ZERO).length();
                let lod = mesh::lod_for_distance(distance, self.detail_distance);
                lods[cz * chunks_x + cx] = Some(mesh::drawn_lod(&data, cx, cz, lod));
            }
        }
        let lod_at = |cx: isize, cz: isize| {
            if cx < 0 || cz < 0 || cx as usize >= chunks_x || cz as usize >= chunks_z {
                return None;
            }
            lods[cz as usize * chunks_x + cx as usize]
        };
        for cz in 0..chunks_z {
            for cx in 0..chunks_x {
                let Some(lod) = lods[cz * chunks_x + cx] else {
                    continue;
                };
                let neighbours =
                    mesh::SIDES.map(|(dx, dz)| lod_at(cx as isize + dx, cz as isize + dz));
                let sides = mesh::side_lods(lod, neighbours);
                let chunk = entry.chunks.entry((cx, cz)).or_default();
                let version = data.chunk_height_version(cx, cz);
                let key = WeightsKey {
                    heights: version,
                    paint: data.chunk_paint_version(cx, cz),
                    rules: rules_key,
                };
                let weights = if background {
                    // Streaming makes missing weights on a thread of its
                    // own, and until then draws the chunk with the weights
                    // it had (or not at all yet).
                    let up_to_date = chunk
                        .weights
                        .as_ref()
                        .is_some_and(|(built, _)| *built == key);
                    let job = (id, cx, cz);
                    if !up_to_date && !self.pending.contains(&job) {
                        let worker = self.chunk_worker.get_or_insert_with(ChunkWorker::start);
                        let sent = worker.jobs.send(ChunkJob {
                            id,
                            uid: data.uid(),
                            cx,
                            cz,
                            weights_key: key,
                            data: data.clone(),
                            placement,
                            rules: rules.clone(),
                        });
                        if sent.is_ok() {
                            self.pending.insert(job);
                        }
                    }
                    let Some((_, weights)) = chunk.weights.as_ref() else {
                        continue;
                    };
                    weights.clone()
                } else {
                    match &chunk.weights {
                        Some((built, weights)) if *built == key => weights.clone(),
                        _ => {
                            let weights =
                                weight_textures(&data, &placement, &rules, (cx, cz), id, key);
                            chunk.weights = Some((key, weights.clone()));
                            weights
                        }
                    }
                };
                // The mesh is quick to build, so it's always the one this
                // frame's levels call for: never one its neighbours don't
                // meet.
                let mesh = match &chunk.meshes[lod as usize] {
                    Some((built, built_sides, mesh))
                        if *built == version && *built_sides == sides =>
                    {
                        mesh.clone()
                    }
                    _ => {
                        let mesh = Arc::new(ChunkSource {
                            key: chunk_key(id, cx, cz, lod, sides),
                            version,
                            mesh: mesh::chunk_mesh(&data, &placement, cx, cz, lod, sides),
                        });
                        chunk.meshes[lod as usize] = Some((version, sides, mesh.clone()));
                        mesh
                    }
                };
                items.push(chunk_item(mesh, weights, &layers, transform, casts_shadow));
            }
        }
        let water = crate::terrain_water::TerrainWater::of(instance);
        crate::terrain_water::draw(id, &water, &data, &placement, &mut entry.water, items);
        crate::foliage::draw_terrain(&mut entry.foliage, id, &placement, &data, assets, chunks);
    }
}

/// A chunk's draw item.
fn chunk_item(
    mesh: Arc<ChunkSource>,
    weights: [Arc<dyn TextureSource>; 2],
    layers: &Arc<TerrainLayers>,
    transform: Mat4,
    casts_shadow: bool,
) -> DrawItem {
    DrawItem {
        shape: Shape::Mesh(mesh),
        transform,
        material: Material {
            terrain: Some(TerrainSurface {
                layers: layers.clone(),
                weights,
            }),
            ..Material::default()
        },
        uv_tile: 0.0,
        casts_shadow,
        id: 0,
        cull_distance: 0.0,
        skin: None,
        sway: None,
        decal_role: Default::default(),
    }
}

/// A chunk mesh's key: the same for the same chunk, level of detail and
/// sides' levels of the same terrain, so the renderer updates it in place
/// when its heights change. Bit 62 is set and 63 clear, so it never
/// matches a model's or a posed model's key.
fn chunk_key(id: InstanceId, cx: usize, cz: usize, lod: u32, sides: [u32; 4]) -> u64 {
    let mut hasher = DefaultHasher::new();
    ("terrain chunk", id, cx, cz, lod, sides).hash(&mut hasher);
    (hasher.finish() & !(1 << 63)) | 1 << 62
}

/// The two weight textures of chunk (`cx`, `cz`): each sample's final
/// weights (rules blended with paint) for layers 1 to 4 and 5 to 8.
fn weight_textures(
    data: &TerrainData,
    placement: &Placement,
    rules: &MaterialRules,
    (cx, cz): (usize, usize),
    id: InstanceId,
    key: WeightsKey,
) -> [Arc<dyn TextureSource>; 2] {
    let rect = data.chunk_rect(cx, cz);
    let (width, depth) = (rect.width(), rect.depth());
    let mut low = Vec::with_capacity(width * depth * 4);
    let mut high = Vec::with_capacity(width * depth * 4);
    for z in rect.z0..rect.z1 {
        for x in rect.x0..rect.x1 {
            let weights = materials::sample_weights(data, placement, rules, x, z);
            let bytes = weights.map(|w| (w.clamp(0.0, 1.0) * 255.0).round() as u8);
            low.extend_from_slice(&bytes[..4]);
            high.extend_from_slice(&bytes[4..]);
        }
    }
    let texture_key = |half: u8| {
        let mut hasher = DefaultHasher::new();
        ("terrain weights", id, data.uid(), cx, cz, key, half).hash(&mut hasher);
        hasher.finish()
    };
    let size = (width as u32, depth as u32);
    [
        Arc::new(WeightTexture {
            key: texture_key(0),
            size,
            rgba: low,
        }),
        Arc::new(WeightTexture {
            key: texture_key(1),
            size,
            rgba: high,
        }),
    ]
}

/// How the Terrain `id`'s layers are drawn.
fn layers_of(
    world: &World,
    id: InstanceId,
    placement: &Placement,
    data: &TerrainData,
    assets: &Assets,
    view: TerrainView,
) -> TerrainLayers {
    let layers: Vec<TerrainLayer> = materials::layers(world, id)
        .into_iter()
        .enumerate()
        .filter_map(|(index, layer)| Some(terrain_layer(world.get(layer)?, assets, index)))
        .collect();
    let cell = placement.cell_size(data);
    let corner = placement.position - placement.size / 2.0;
    let mut hasher = DefaultHasher::new();
    view.hash(&mut hasher);
    for number in [
        placement.position.x,
        placement.position.y,
        placement.position.z,
        placement.size.x,
        placement.size.y,
        placement.size.z,
        cell.x,
        cell.y,
    ] {
        number.to_bits().hash(&mut hasher);
    }
    for layer in materials::layers(world, id) {
        if let Some(instance) = world.get(layer) {
            for (name, value) in instance.properties() {
                name.hash(&mut hasher);
                format!("{value:?}").hash(&mut hasher);
            }
        }
    }
    // Textures that load (or fail) later change what's drawn too.
    for layer in &layers {
        let keys = [
            layer.base_color_texture.as_ref().map(|t| t.key()),
            layer.normal.as_ref().map(|m| m.texture.key()),
            layer.roughness_map.as_ref().map(|m| m.texture.key()),
            layer.metalness_map.as_ref().map(|m| m.texture.key()),
            layer.occlusion.as_ref().map(|m| m.texture.key()),
            layer.height.as_ref().map(|m| m.texture.key()),
        ];
        keys.hash(&mut hasher);
    }
    let mut id_hasher = DefaultHasher::new();
    ("terrain layers", id).hash(&mut id_hasher);
    TerrainLayers {
        id: id_hasher.finish(),
        key: hasher.finish(),
        layers,
        view,
        height_range: [
            placement.position.y,
            placement.position.y + placement.size.y,
        ],
        grid_origin: [corner.x, corner.z],
        grid_cell: [cell.x, cell.y],
    }
}

/// A TerrainLayer Instance as the renderer takes it. Its textures come
/// from `assets` (a missing file is reported there, once, and left out).
fn terrain_layer(layer: &Instance, assets: &Assets, index: usize) -> TerrainLayer {
    let texture_of = |name| {
        let path = layer.property(name).and_then(Value::as_str)?;
        if path.is_empty() {
            return None;
        }
        assets.texture(path).map(|image| image_texture(&image))
    };
    let number = |name, default: f32| {
        layer
            .property(name)
            .and_then(Value::as_number)
            .map_or(default, |n| n as f32)
    };
    let flag = |name| layer.property(name).and_then(Value::as_bool) == Some(true);
    let channel = |name| match layer.property(name).and_then(Value::as_texture_channel) {
        Some(TextureChannel::Green) => Channel::Green,
        Some(TextureChannel::Blue) => Channel::Blue,
        Some(TextureChannel::Alpha) => Channel::Alpha,
        Some(TextureChannel::Red) | None => Channel::Red,
    };
    let map = |name, channel, invert| {
        texture_of(name).map(|texture| ChannelMap {
            texture,
            channel,
            invert,
        })
    };
    let color = layer
        .property("Color")
        .and_then(Value::as_color3)
        .map_or([1.0; 3], |c| [c.r, c.g, c.b]);
    TerrainLayer {
        color,
        base_color_texture: texture_of("ColorMap"),
        normal: texture_of("NormalMap").map(|texture| NormalMap {
            texture,
            scale: number("NormalStrength", 1.0),
            flip_green: flag("NormalMapDirectX"),
        }),
        metalness_map: map("MetalnessMap", channel("MetalnessChannel"), false),
        roughness_map: map(
            "RoughnessMap",
            channel("RoughnessChannel"),
            flag("InvertRoughness"),
        ),
        occlusion: map(
            "AmbientOcclusionMap",
            channel("AmbientOcclusionChannel"),
            false,
        ),
        height: map("HeightMap", Channel::Red, false),
        metalness: number("Metalness", 0.0).clamp(0.0, 1.0),
        roughness: number("Roughness", 0.9).clamp(0.0, 1.0),
        tile_size: number("TileSize", 8.0).max(0.05),
        rotation_degrees: number("TextureRotation", 0.0),
        triplanar: flag("Triplanar"),
        anti_tiling: number("AntiTiling", 0.3).clamp(0.0, 1.0),
        height_blend: number("HeightBlend", 0.5).clamp(0.0, 1.0),
        debug_color: DEBUG_COLORS[index % MAX_TERRAIN_LAYERS],
    }
}

/// A chunk's mesh, handed to the renderer without copying it.
struct ChunkSource {
    key: u64,
    version: u64,
    mesh: ChunkMesh,
}

impl MeshSource for ChunkSource {
    fn key(&self) -> u64 {
        self.key
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.mesh.positions
    }
    fn normals(&self) -> &[[f32; 3]] {
        &self.mesh.normals
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &self.mesh.uvs
    }
    fn indices(&self) -> &[u32] {
        &self.mesh.indices
    }
    fn version(&self) -> u64 {
        self.version
    }
}

struct WeightTexture {
    key: u64,
    size: (u32, u32),
    rgba: Vec<u8>,
}

impl TextureSource for WeightTexture {
    fn key(&self) -> u64 {
        self.key
    }
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Where `ray` first hits a Workspace Terrain: the Terrain, how far along
/// the ray (studs), and which way the ground faces there.
pub fn terrain_hit(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    skip: &[InstanceId],
) -> Option<(InstanceId, f32, Vec3)> {
    let mut nearest: Option<(InstanceId, f32, Vec3)> = None;
    for id in world.all_below(world.workspace(), |class| {
        class == axcenity_core::ClassName::Terrain
    }) {
        if skip.contains(&id) {
            continue;
        }
        let Some(instance) = world.get(id) else {
            continue;
        };
        let (Some(placement), Some(data)) = (Placement::of(instance), world.terrain(id)) else {
            continue;
        };
        let Some(distance) =
            axcenity_terrain::raycast::raycast(data, &placement, origin, direction)
        else {
            continue;
        };
        if nearest.is_some_and(|(_, nearest, _)| nearest <= distance) {
            continue;
        }
        let point = origin + direction * distance;
        let grid = placement.to_grid(data, point.x, point.z).round();
        let (size_x, size_z) = data.size();
        let normal = placement.normal(
            data,
            (grid.x.max(0.0) as usize).min(size_x - 1),
            (grid.y.max(0.0) as usize).min(size_z - 1),
        );
        nearest = Some((id, distance, normal));
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_core::{ClassName, Color3};

    fn camera_at(position: Vec3) -> Camera {
        Camera {
            position,
            look_at: Vec3::ZERO,
            fov_y_degrees: 70.0,
        }
    }

    fn world_with_terrain() -> (World, InstanceId) {
        let mut world = World::new();
        let terrain = world.create(ClassName::Terrain).unwrap();
        world.set_parent(terrain, Some(world.workspace())).unwrap();
        for color in [Color3::new(0.3, 0.6, 0.2), Color3::new(0.5, 0.5, 0.5)] {
            let layer = world.create(ClassName::TerrainLayer).unwrap();
            world.set_parent(layer, Some(terrain)).unwrap();
            world
                .set_property(layer, "Color", Value::Color3(color))
                .unwrap();
        }
        (world, terrain)
    }

    fn chunk_versions(items: &[DrawItem]) -> Vec<(u64, u64)> {
        items
            .iter()
            .filter_map(|item| match &item.shape {
                Shape::Mesh(mesh) if item.material.terrain.is_some() => {
                    Some((mesh.key(), mesh.version()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_terrain_draws_one_item_per_chunk_with_its_layers() {
        let (world, _) = world_with_terrain();
        let mut cache = TerrainCache::new();
        let mut items = Vec::new();
        cache.draw_all(
            &world,
            &Assets::default(),
            &camera_at(Vec3::new(0.0, 50.0, 0.0)),
            &mut items,
            &mut Vec::new(),
        );
        // 257 samples is 8 × 8 chunks.
        assert_eq!(items.len(), 64);
        let surface = items[0].material.terrain.as_ref().unwrap();
        assert_eq!(surface.layers.layers.len(), 2);
        assert_eq!(surface.layers.layers[0].color, [0.3, 0.6, 0.2]);
        assert_eq!(
            items[0].transform,
            Mat4::from_translation(Vec3::new(0.0, -32.0, 0.0))
        );
        // Nothing painted, no rules: all the first layer.
        assert_eq!(&surface.weights[0].rgba()[..4], &[255, 0, 0, 0]);
    }

    #[test]
    fn an_edit_rebuilds_only_the_chunks_it_touched() {
        let (mut world, terrain) = world_with_terrain();
        let mut cache = TerrainCache::new();
        let assets = Assets::default();
        let camera = camera_at(Vec3::new(0.0, 2000.0, 0.0));
        let mut before = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut before, &mut Vec::new());
        let data = world.terrain_mut(terrain).unwrap();
        let middle = axcenity_core::SampleRect::new(48, 48, 49, 49);
        data.heights_mut_in(middle)[48 * 257 + 48] = 0.9;
        let mut after = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut after, &mut Vec::new());
        let (before, after) = (chunk_versions(&before), chunk_versions(&after));
        let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
        assert_eq!(changed, 1, "one chunk's mesh is rebuilt");
        // Its key stays, so the renderer updates it in place.
        assert!(before.iter().zip(&after).all(|(a, b)| a.0 == b.0));
    }

    #[test]
    fn nearby_chunks_get_full_detail_and_far_ones_less() {
        let (world, _) = world_with_terrain();
        let mut cache = TerrainCache::new();
        let mut items = Vec::new();
        // Low over one corner of the 512-stud terrain.
        cache.draw_all(
            &world,
            &Assets::default(),
            &camera_at(Vec3::new(-250.0, 10.0, -250.0)),
            &mut items,
            &mut Vec::new(),
        );
        let corners = |item: &DrawItem| match &item.shape {
            Shape::Mesh(mesh) => mesh.positions().len(),
            _ => 0,
        };
        let near = corners(&items[0]);
        let far = corners(&items[63]);
        assert!(near > far * 4, "{near} {far}");
    }

    #[test]
    fn moving_the_camera_draws_every_chunk_every_frame_with_closed_seams() {
        let (mut world, terrain) = world_with_terrain();
        let data = world.terrain_mut(terrain).unwrap();
        let (size_x, _) = data.size();
        let all = data.bounds();
        for (i, h) in data.heights_mut_in(all).iter_mut().enumerate() {
            let (x, z) = ((i % size_x) as f32, (i / size_x) as f32);
            *h = 0.4 + 0.2 * (x * 0.05).sin() * (z * 0.07).cos();
        }
        let mut cache = TerrainCache::new();
        cache.detail_distance = 40.0;
        let assets = Assets::default();
        let mut levels_seen = HashSet::new();
        // Low over the 512-stud terrain from one corner to the other: the
        // chunks change level as the camera passes.
        for step in 0..=40 {
            let along = -300.0 + step as f32 * 15.0;
            let mut items = Vec::new();
            cache.draw_all(
                &world,
                &assets,
                &camera_at(Vec3::new(along, 20.0, along * 0.7)),
                &mut items,
                &mut Vec::new(),
            );
            let chunks: Vec<&Arc<dyn MeshSource>> = items
                .iter()
                .filter(|item| item.material.terrain.is_some())
                .filter_map(|item| match &item.shape {
                    Shape::Mesh(mesh) => Some(mesh),
                    _ => None,
                })
                .collect();
            assert_eq!(chunks.len(), 64, "every chunk is drawn at step {step}");
            levels_seen.extend(chunks.iter().map(|mesh| mesh.positions().len()));
            // Every edge inside the terrain has a triangle on each side,
            // wherever two levels meet: nothing to see through.
            let mut uses: HashMap<[[u32; 3]; 2], usize> = HashMap::new();
            for mesh in &chunks {
                let corner = |k: u32| mesh.positions()[k as usize].map(f32::to_bits);
                for triangle in mesh.indices().chunks(3) {
                    for k in 0..3 {
                        let (a, b) = (corner(triangle[k]), corner(triangle[(k + 1) % 3]));
                        *uses.entry(if a < b { [a, b] } else { [b, a] }).or_default() += 1;
                    }
                }
            }
            for (edge, count) in uses {
                let [a, b] = edge.map(|p| p.map(f32::from_bits));
                let border =
                    (a[0] == b[0] && a[0].abs() == 256.0) || (a[2] == b[2] && a[2].abs() == 256.0);
                assert_eq!(
                    count,
                    if border { 1 } else { 2 },
                    "step {step}: {a:?} to {b:?}"
                );
            }
        }
        assert!(levels_seen.len() > 3, "the chunks changed level");
    }

    #[test]
    fn an_override_is_drawn_instead_and_rays_hit_the_ground() {
        let (world, terrain) = world_with_terrain();
        let mut cache = TerrainCache::new();
        let mut flat = TerrainData::flat(65, 65, 0.0).unwrap();
        let all = flat.bounds();
        flat.heights_mut_in(all)[0] = 0.5;
        cache.set_override(terrain, Some(Arc::new(flat)), None);
        let mut items = Vec::new();
        cache.draw_all(
            &world,
            &Assets::default(),
            &camera_at(Vec3::new(0.0, 50.0, 0.0)),
            &mut items,
            &mut Vec::new(),
        );
        assert_eq!(items.len(), 4);
        cache.set_override(terrain, None, None);
        assert!(!cache.has_override(terrain));

        // The world's own terrain is flat at 0 (a quarter of 128, from -32).
        let (hit, distance, normal) =
            terrain_hit(&world, Vec3::new(10.0, 100.0, 10.0), Vec3::NEG_Y, &[]).unwrap();
        assert_eq!(hit, terrain);
        assert!((distance - 100.0).abs() < 0.01, "{distance}");
        assert!((normal - Vec3::Y).length() < 1e-5);
        assert!(
            terrain_hit(
                &world,
                Vec3::new(10.0, 100.0, 10.0),
                Vec3::NEG_Y,
                &[terrain]
            )
            .is_none()
        );
    }

    fn water_items(items: &[DrawItem]) -> Vec<&DrawItem> {
        items
            .iter()
            .filter(|item| item.material.water.is_some())
            .collect()
    }

    #[test]
    fn terrain_water_fills_only_the_chunks_whose_ground_dips_below_its_level() {
        let (mut world, terrain) = world_with_terrain();
        let ground = {
            let data = world.terrain(terrain).unwrap();
            -32.0 + data.height(48, 48) * 128.0
        };
        world
            .set_property(terrain, "Water", Value::Bool(true))
            .unwrap();
        world
            .set_property(
                terrain,
                "WaterLevel",
                Value::Number(f64::from(ground) - 4.0),
            )
            .unwrap();
        world
            .set_property(terrain, "WaveHeight", Value::Number(1.5))
            .unwrap();
        let mut cache = TerrainCache::new();
        let assets = Assets::default();
        let camera = camera_at(Vec3::new(0.0, 50.0, 0.0));
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        assert!(
            water_items(&items).is_empty(),
            "the ground is above the water everywhere"
        );

        // A hole in chunk (1, 1): only that chunk floods.
        let data = world.terrain_mut(terrain).unwrap();
        let hole = axcenity_core::SampleRect::new(48, 48, 49, 49);
        data.heights_mut_in(hole)[48 * 257 + 48] = 0.0;
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        let water = water_items(&items);
        assert_eq!(water.len(), 1);
        assert_eq!(water[0].material.water.unwrap().wave_height, 1.5);
        // 32 cells of 2 studs, from x = z = -256 + 64; its top at the level.
        let (scale, _, middle) = water[0].transform.to_scale_rotation_translation();
        assert_eq!(scale, Vec3::new(64.0, 1.0, 64.0));
        assert_eq!(middle, Vec3::new(-160.0, ground - 4.5, -160.0));
        assert!(!water[0].casts_shadow);

        // Up over all the ground: every chunk has water.
        world
            .set_property(
                terrain,
                "WaterLevel",
                Value::Number(f64::from(ground) + 1.0),
            )
            .unwrap();
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        assert_eq!(water_items(&items).len(), 64);

        // Off: none.
        world
            .set_property(terrain, "Water", Value::Bool(false))
            .unwrap();
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        assert!(water_items(&items).is_empty());
    }

    #[test]
    fn painted_water_draws_only_its_cells_that_show() {
        let (mut world, terrain) = world_with_terrain();
        let ground = {
            let data = world.terrain(terrain).unwrap();
            -32.0 + data.height(48, 48) * 128.0
        };
        // Four cells in chunk (1, 1), 3 studs over the ground, and one in
        // chunk (0, 0) far under it.
        let level = (ground + 3.0 + 32.0) / 128.0;
        let data = world.terrain_mut(terrain).unwrap();
        let all = data.bounds();
        let water = data.water_mut_in(all);
        for (x, z) in [(48, 48), (49, 48), (48, 49), (49, 49)] {
            water[z * 257 + x] = level;
        }
        water[5 * 257 + 5] = 0.0;
        world
            .set_property(terrain, "WaveHeight", Value::Number(0.75))
            .unwrap();
        let mut cache = TerrainCache::new();
        let assets = Assets::default();
        let camera = camera_at(Vec3::new(0.0, 50.0, 0.0));
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        let water = water_items(&items);
        assert_eq!(water.len(), 1, "the dry cell isn't drawn");
        assert_eq!(water[0].material.water.unwrap().wave_height, 0.75);
        let Shape::Mesh(mesh) = &water[0].shape else {
            panic!("painted water is a mesh of its cells");
        };
        // Four cells, each cut 2 × 2 (32 cells a chunk, 64 in the grid).
        assert_eq!(mesh.indices().len(), 4 * 2 * 2 * 6);
        assert!(
            mesh.positions()
                .iter()
                .all(|p| (p[1] - (ground + 3.0)).abs() < 1e-3)
        );
        let key = mesh.key();
        // Drawn again unchanged: the same mesh.
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        let Shape::Mesh(again) = &water_items(&items)[0].shape else {
            unreachable!()
        };
        assert_eq!(again.key(), key);

        // Flooded higher: the flood covers it, so only the flood draws.
        world
            .set_property(terrain, "Water", Value::Bool(true))
            .unwrap();
        world
            .set_property(
                terrain,
                "WaterLevel",
                Value::Number(f64::from(ground) + 5.0),
            )
            .unwrap();
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        assert!(
            water_items(&items)
                .iter()
                .all(|item| matches!(&item.shape, Shape::Mesh(mesh) if mesh.key() == axcenity_render::water_surface().key()))
        );
    }

    #[test]
    fn painted_water_feathers_exposed_outer_edges_and_blends_adjacent_levels() {
        let (mut world, terrain) = world_with_terrain();
        let ground = {
            let data = world.terrain(terrain).unwrap();
            -32.0 + data.height(48, 48) * 128.0
        };
        let low = (ground + 2.0 + 32.0) / 128.0;
        let high = (ground + 4.0 + 32.0) / 128.0;
        let data = world.terrain_mut(terrain).unwrap();
        let all = data.bounds();
        let water = data.water_mut_in(all);
        // 2x2 block of cells: left column at `low`, right column at `high`.
        water[48 * 257 + 48] = low;
        water[49 * 257 + 48] = low;
        water[48 * 257 + 49] = high;
        water[49 * 257 + 49] = high;

        let mut cache = TerrainCache::new();
        let assets = Assets::default();
        let camera = camera_at(Vec3::new(0.0, 50.0, 0.0));
        let mut items = Vec::new();
        cache.draw_all(&world, &assets, &camera, &mut items, &mut Vec::new());
        let water = water_items(&items);
        assert_eq!(water.len(), 1);
        let Shape::Mesh(mesh) = &water[0].shape else {
            panic!("expected painted water mesh");
        };
        // Center vertex (shared by all 4 cells) must have full edge mask (-1.0)
        // and the averaged height of low and high (ground + 3.0).
        let has_interior_averaged = mesh
            .positions()
            .iter()
            .zip(mesh.tangents())
            .any(|(p, t)| (p[1] - (ground + 3.0)).abs() < 1e-2 && (t[3] - (-1.0)).abs() < 1e-4);
        assert!(
            has_interior_averaged,
            "shared interior corner averages adjacent levels and keeps full opacity"
        );
        // Exposed outer border vertices must carry tangent.w == -2.0 (edge_mask 0.0).
        let has_feathered_border = mesh.tangents().iter().any(|t| (t[3] - (-2.0)).abs() < 1e-4);
        assert!(
            has_feathered_border,
            "exposed unpainted border vertices are feathered to 0"
        );
    }
}
