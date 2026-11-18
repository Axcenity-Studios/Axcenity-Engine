//! Compute posing (CO8, ADR-0041 decision 2, `docs/rendering/animation.md`).
//!
//! Each frame, every skinned or morphed item a pass will draw (or ray
//! tracing will trace) is leaned toward its blend shapes and moved by its
//! joints **once**, by a compute pass (`posing.wgsl`), into the frame's
//! posed buffer. The vertex shaders of every pass (shadow views, depth
//! prepass, motion vectors, scene, outlines) read their corners from it
//! (`pose_corner` in shader.wgsl), and ray tracing builds the item's BLAS
//! from it, so ray traced shadows, AO, reflections and GI see the shapes
//! too.
//!
//! - **Pairs.** What is posed is a (skin, mesh) pair: items sharing a
//!   [`Skin`] and a mesh share one posed copy. Each pair gets a range of
//!   the posed buffer (its mesh's corner count), and its instance's `skin`
//!   says where it starts (`[first corner, 1]`; `[0, 0]` is not posed).
//! - **Active shapes.** Only shapes whose weight isn't 0 are listed for a
//!   pair ([`active_shapes`]), and the pass walks only their moves: a shape
//!   a description never moves costs nothing. Shapes are uploaded shape by
//!   shape for this (`skinning.rs`).
//! - **Last frame.** While motion vectors are drawn, last frame's posed
//!   buffer is copied aside before this frame's posing, and the motion
//!   pass reads each item's old corners there (`History` keeps where).
//! - **The arena.** Every posed mesh's corners and joints and weights, and
//!   the moves of each of its shapes that has been active, are taken up
//!   into three shared buffers ([`Arena`]), so the whole frame is posed in
//!   two dispatches, however many meshes it has.
//! - **No fallback.** Compute shaders, storage buffers in vertex shaders
//!   and storage atomics are all core WebGPU, so every device this renderer
//!   opens can pose; there is no per-pass or CPU path left.

use bytemuck::{Pod, Zeroable};
#[cfg(test)]
use glam::Vec3;
use rustc_hash::FxHashMap;

use super::gpu::shader_module;
use super::{GpuMesh, MeshKey};
use crate::draw_list::Skin;

/// What the posing pass did in the last frame, for [`crate::RenderStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PosingStats {
    /// (Skin, mesh) pairs posed: one per skinned or morphed item drawn,
    /// unless items share a skin and a mesh.
    pub items: u32,
    /// Their corners.
    pub corners: u64,
    /// Active shapes (weight other than 0), summed over the items.
    pub shapes: u32,
    /// Shape moves (one corner moved by one shape) the pass applied.
    pub moves: u64,
    /// Bytes of the posing buffers: this frame's posed corners, last
    /// frame's copy (while motion vectors are drawn), the shape sums and
    /// the item and shape lists.
    pub bytes: u64,
}

/// Where one blend shape's moves are among its mesh's, and which of a
/// skin's weights drives it ([`crate::MorphView::target`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ShapeRange {
    pub(super) first: u32,
    pub(super) count: u32,
    pub(super) target: u32,
}

/// One shape a posed item leans toward this frame. Must match `Active` in
/// posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct ActiveShape {
    /// Its moves: among the mesh's ([`active_shapes`]), then, on the GPU,
    /// in the posing arena.
    pub(super) first: u32,
    pub(super) count: u32,
    /// How many moves of the item's active shapes come before it.
    pub(super) start: u32,
    pub(super) weight: f32,
}

/// The shapes of `shapes` that `weights` (a skin's, by target) lean the
/// mesh toward: those with a weight other than 0 (and a number), in order,
/// each after the moves of those before it. A shape at weight 0, or whose
/// target has no weight, isn't listed, so it costs the pass nothing.
#[cfg(test)]
pub(super) fn active_shapes(shapes: &[ShapeRange], weights: &[f32]) -> Vec<ActiveShape> {
    active_shape_list(shapes, weights)
        .into_iter()
        .map(|(_, shape)| shape)
        .collect()
}

/// [`active_shapes`], each with its index among `shapes`.
fn active_shape_list(shapes: &[ShapeRange], weights: &[f32]) -> Vec<(u32, ActiveShape)> {
    let mut start = 0;
    shapes
        .iter()
        .enumerate()
        .filter_map(|(index, shape)| {
            let weight = *weights.get(shape.target as usize)?;
            if weight == 0.0 || !weight.is_finite() || shape.count == 0 {
                return None;
            }
            let active = ActiveShape {
                first: shape.first,
                count: shape.count,
                start,
                weight,
            };
            start += shape.count;
            Some((index as u32, active))
        })
        .collect()
}

/// One item (pair) to pose. Must match `Job` in posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct JobRaw {
    posed: u32,
    corners: u32,
    sums: u32,
    joints: u32,
    joint_count: u32,
    active_first: u32,
    active_count: u32,
    moves: u32,
    moves_start: u32,
    vertices: u32,
    skin: u32,
}

/// The frame's totals. Must match `Totals` in posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct TotalsRaw {
    jobs: u32,
    corners: u32,
    moves: u32,
    stride: u32,
}

/// Words per posed corner. Must match `POSED_WORDS` in posing.wgsl and
/// shader.wgsl.
pub(super) const POSED_WORDS: u64 = 5;
/// Bytes per posed corner: the stride ray tracing reads positions at.
pub(super) const POSED_STRIDE: u64 = POSED_WORDS * 4;
/// Sums per corner: its move and its turn, three each.
const SUMS_PER_CORNER: u64 = 6;
/// No sums, or no joints and weights, in a [`JobRaw`]. Must match `NONE`
/// in posing.wgsl.
const NONE: u32 = u32::MAX;
/// `cs_shapes`' and `cs_pose`'s workgroup sizes.
const SHAPE_GROUP: u32 = 256;
const POSE_GROUP: u32 = 64;
const MOST_GROUPS: u32 = 65_535;

/// The `skin` value an instance of a posed pair waits with in a plan
/// (`[pair, PENDING]`) until its posed range is known.
pub(super) const PENDING: u32 = u32::MAX;

/// One pair a frame wants posed: an item's `skin` drawn with `mesh`, its
/// joints starting at `joints` in the frame's joints buffer.
pub(super) struct PoseRequest<'a> {
    pub(super) skin: &'a Skin,
    pub(super) mesh: MeshKey,
    pub(super) joints: u32,
}

/// Where one mesh is in the arena: its first corner, its first corner's
/// joints and weights ([`NONE`] without joints), and by shape index where
/// each shape's moves start ([`NONE`] for one not taken up yet).
#[derive(Clone, Debug)]
struct Source {
    vertices: u32,
    skin: u32,
    shapes: Vec<u32>,
}

/// A mesh by key and version, which names its data in the arena.
type MeshVersion = (MeshKey, u64);

/// One mesh and the active shapes (by index) an item of it uses, as the
/// arena is asked for them.
struct Wanted<'a> {
    key: MeshKey,
    gpu: &'a GpuMesh,
    shapes: &'a [(u32, ActiveShape)],
}

/// Every posed mesh's data in three buffers, so one dispatch poses every
/// item of a frame whatever its mesh: its corners (copied from the mesh's
/// vertex buffer), their joints and weights, and the moves of each of its
/// shapes that has been active. A mesh goes in the first time it's posed
/// (by key and version), and a shape the first time it has a weight, so a
/// shape no item ever uses takes no room. When something doesn't fit, the
/// arena is filled afresh with the frame's own, with room to spare.
struct Arena {
    vertices: wgpu::Buffer,
    skin: wgpu::Buffer,
    moves: wgpu::Buffer,
    /// How many corners, joint-and-weight entries and moves are in use.
    used: [u64; 3],
    sources: FxHashMap<MeshVersion, Source>,
    /// Corners to copy in when the frame is recorded: the mesh, and where.
    copies: Vec<(MeshKey, u32)>,
    /// Whether a frame has been too big to fit (warned once).
    warned: bool,
}

/// Sizes of one arena entry, by buffer: a `Vertex`, a `SkinCorner`, a
/// `MoveRaw`.
const ARENA_SIZES: [u64; 3] = [48, 32, 32];
const ARENA_LABELS: [&str; 3] = [
    "posing arena corners",
    "posing arena joints and weights",
    "posing arena moves",
];
/// Arena buffers grow in steps of this many bytes.
const ARENA_STEP: u64 = 1 << 20;

impl Arena {
    fn new(device: &wgpu::Device) -> Self {
        let make = |index: usize| arena_buffer(device, index, 64);
        Self {
            vertices: make(0),
            skin: make(1),
            moves: make(2),
            used: [0; 3],
            sources: FxHashMap::default(),
            copies: Vec::new(),
            warned: false,
        }
    }

    fn buffers(&self) -> [&wgpu::Buffer; 3] {
        [&self.vertices, &self.skin, &self.moves]
    }

    fn bytes(&self) -> u64 {
        self.buffers().iter().map(|buffer| buffer.size()).sum()
    }

    /// Takes up each mesh and shape of `wanted` that isn't in yet. If they
    /// don't all fit, the arena is filled afresh with just `wanted`, grown
    /// to half again what they take (up to what a binding may hold: past
    /// that, the rest is left out and their items are drawn at rest, with a
    /// warning).
    fn take_up(&mut self, (device, queue): (&wgpu::Device, &wgpu::Queue), wanted: &[Wanted]) {
        if self.place(Some(queue), wanted, false) {
            return;
        }
        // Afresh: how much `wanted` takes alone, then room for it.
        self.reset();
        self.place(None, wanted, true);
        let all = self.used;
        self.reset();
        let capacity = self.buffers().map(|buffer| buffer.size());
        let limits = device.limits();
        let most = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        for index in 0..3 {
            let bytes = all[index] * ARENA_SIZES[index];
            let room = (bytes + bytes / 2).next_multiple_of(ARENA_STEP).min(most);
            if room > capacity[index] || room < capacity[index] / 4 {
                let fresh = arena_buffer(device, index, room);
                match index {
                    0 => self.vertices = fresh,
                    1 => self.skin = fresh,
                    _ => self.moves = fresh,
                }
            }
        }
        if !self.place(Some(queue), wanted, false) && !self.warned {
            self.warned = true;
            log::warn!(
                "too many animated meshes and shapes to pose in one frame (past what one storage \
                 buffer binding may hold); the rest are drawn at rest"
            );
        }
    }

    fn reset(&mut self) {
        self.sources.clear();
        self.copies.clear();
        self.used = [0; 3];
    }

    /// Puts what of `wanted` isn't in yet after what is, writing it with
    /// `queue` (or, without one, only counting it). False if something
    /// didn't fit (unless `unbounded`); what did is in.
    fn place(&mut self, queue: Option<&wgpu::Queue>, wanted: &[Wanted], unbounded: bool) -> bool {
        let capacity = self.buffers().map(|buffer| buffer.size());
        let fits = |used: u64, more: u64, index: usize| {
            unbounded || (used + more) * ARENA_SIZES[index] <= capacity[index]
        };
        let mut all_in = true;
        for want in wanted {
            let skin = want.gpu.skin.as_ref().expect("animated meshes only");
            let version = (want.key, want.gpu.version);
            if !self.sources.contains_key(&version) {
                let [vertices, corners, _] = self.used;
                let more = (u64::from(want.gpu.vertex_count), skin.corners.len() as u64);
                if !fits(vertices, more.0, 0) || !fits(corners, more.1, 1) {
                    all_in = false;
                    continue;
                }
                if let Some(queue) = queue {
                    self.copies.push((want.key, vertices as u32));
                    if !skin.corners.is_empty() {
                        queue.write_buffer(
                            &self.skin,
                            corners * ARENA_SIZES[1],
                            bytemuck::cast_slice(&skin.corners),
                        );
                    }
                }
                self.sources.insert(
                    version,
                    Source {
                        vertices: vertices as u32,
                        skin: if skin.corners.is_empty() {
                            NONE
                        } else {
                            corners as u32
                        },
                        shapes: vec![NONE; skin.shapes.len()],
                    },
                );
                self.used[0] += more.0;
                self.used[1] += more.1;
            }
            let source = self.sources.get_mut(&version).expect("put in above");
            for &(shape, _) in want.shapes {
                if source.shapes[shape as usize] != NONE {
                    continue;
                }
                let range = skin.shapes[shape as usize];
                let count = u64::from(range.count);
                if !fits(self.used[2], count, 2) {
                    all_in = false;
                    continue;
                }
                if let Some(queue) = queue {
                    let first = range.first as usize;
                    queue.write_buffer(
                        &self.moves,
                        self.used[2] * ARENA_SIZES[2],
                        bytemuck::cast_slice(&skin.moves[first..first + range.count as usize]),
                    );
                }
                source.shapes[shape as usize] = self.used[2] as u32;
                self.used[2] += count;
            }
        }
        all_in
    }
}

fn arena_buffer(device: &wgpu::Device, index: usize, bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(ARENA_LABELS[index]),
        size: bytes.max(64).next_multiple_of(16),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// The posing pass and its buffers.
pub(super) struct Posing {
    shapes_pipeline: wgpu::ComputePipeline,
    pose_pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    group: Option<wgpu::BindGroup>,
    totals: wgpu::Buffer,
    jobs: wgpu::Buffer,
    active: wgpu::Buffer,
    sums: wgpu::Buffer,
    arena: Arena,
    /// This frame's posed corners ([`POSED_WORDS`] each), bound as the
    /// frame's binding 9.
    posed: wgpu::Buffer,
    /// Last frame's, copied aside while motion vectors are drawn.
    previous: wgpu::Buffer,
    /// The posed buffer last frame was written to, when this frame's had
    /// to grow: last frame's corners are copied from it.
    retired: Option<wgpu::Buffer>,
    /// Corners last frame posed, and this frame, and this frame's moves.
    last_corners: u64,
    corners: u64,
    moves: u64,
    copy_previous: bool,
    /// Where each pair's corners start in `posed`, this frame, by the
    /// skin's address (only compared, never followed, so the pass stays
    /// `Send`) and the mesh.
    offsets: FxHashMap<(usize, MeshKey), u32>,
    /// Counts up whenever `posed` or `previous` is made again, so the
    /// bindings holding them are made again too.
    pub(super) generation: u64,
    stats: PosingStats,
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(64).next_power_of_two(),
        usage,
        mapped_at_creation: false,
    })
}

fn posed_usage(device: &wgpu::Device) -> wgpu::BufferUsages {
    wgpu::BufferUsages::STORAGE
        | wgpu::BufferUsages::COPY_SRC
        | wgpu::BufferUsages::COPY_DST
        | super::ray_tracing::blas_input(device)
}

/// Workgroups for `items` threads in groups of `size`, as rows of at most
/// 65,535 (the x and y of a dispatch).
fn rows(items: u64, size: u32) -> (u32, u32) {
    let groups = items.div_ceil(u64::from(size)).max(1);
    let x = groups.min(u64::from(MOST_GROUPS)) as u32;
    let y = groups.div_ceil(u64::from(MOST_GROUPS)) as u32;
    (x, y)
}

impl Posing {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("posing"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, false),
                storage_entry(5, false),
                storage_entry(6, true),
                storage_entry(7, true),
                storage_entry(8, true),
            ],
        });
        let shader = shader_module(device, "posing", super::gpu::POSING_SHADER);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("posing"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        Self {
            shapes_pipeline: pipeline("cs_shapes"),
            pose_pipeline: pipeline("cs_pose"),
            layout,
            group: None,
            totals: buffer(
                device,
                "posing totals",
                16,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
            jobs: buffer(device, "posed items", 64, storage),
            active: buffer(device, "active shapes", 64, storage),
            sums: buffer(device, "shape sums", 64, wgpu::BufferUsages::STORAGE),
            arena: Arena::new(device),
            posed: buffer(device, "posed corners", 64, posed_usage(device)),
            previous: buffer(
                device,
                "last frame's posed corners",
                64,
                posed_usage(device),
            ),
            retired: None,
            last_corners: 0,
            corners: 0,
            moves: 0,
            copy_previous: false,
            offsets: FxHashMap::default(),
            generation: 0,
            stats: PosingStats::default(),
        }
    }

    /// This frame's posed corners (the frame's binding 9).
    pub(super) fn posed(&self) -> &wgpu::Buffer {
        &self.posed
    }

    /// Last frame's posed corners, where `History` says each item's were.
    pub(super) fn previous(&self) -> &wgpu::Buffer {
        &self.previous
    }

    pub(super) fn stats(&self) -> PosingStats {
        self.stats
    }

    /// Moves the arena holds (for tests): only shapes that have been
    /// active are taken up.
    #[cfg(test)]
    pub(super) fn arena_moves(&self) -> u64 {
        self.arena.used[2]
    }

    /// Where `skin` drawn with `mesh` was posed this frame: its first
    /// corner in the posed buffer, or `None` if it wasn't.
    pub(super) fn offset(&self, skin: &Skin, mesh: MeshKey) -> Option<u32> {
        self.offsets
            .get(&(std::ptr::from_ref(skin).addr(), mesh))
            .copied()
    }

    /// Plans this frame's posing of `requests` (each a different pair,
    /// whose joints are in `joints`, the frame's joints buffer) and uploads
    /// its lists: each pair's first corner in the posed buffer, or `None`
    /// for one that needn't be posed (its mesh isn't animated, or neither
    /// its joints nor its shapes move it) and is drawn at rest, or that
    /// didn't fit the arena. `keep_previous` keeps last frame's corners for
    /// the motion vectors.
    pub(super) fn prepare<'a>(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        requests: &[PoseRequest<'_>],
        mesh: impl Fn(MeshKey) -> Option<&'a GpuMesh>,
        joints: &wgpu::Buffer,
        keep_previous: bool,
    ) -> Vec<Option<u32>> {
        self.last_corners = self.corners;
        self.copy_previous = keep_previous && self.last_corners > 0;
        self.offsets.clear();
        // Each pair (the requests name each once: `pose_frame` lists them
        // so), with its mesh, joints and active shapes.
        struct Candidate<'a> {
            request: usize,
            gpu: &'a GpuMesh,
            joint_count: u32,
            shapes: Vec<(u32, ActiveShape)>,
        }
        let mut candidates: Vec<Candidate<'a>> = Vec::new();
        let mut candidate_of: Vec<Option<usize>> = vec![None; requests.len()];
        for (index, request) in requests.iter().enumerate() {
            let Some(gpu) = mesh(request.mesh) else {
                continue;
            };
            let Some(gpu_skin) = &gpu.skin else {
                continue;
            };
            let joint_count = match gpu_skin.jointed {
                true => request.skin.joints().len().min(0xFFFF) as u32,
                false => 0,
            };
            let shapes = active_shape_list(&gpu_skin.shapes, request.skin.morph_weights());
            if joint_count == 0 && shapes.is_empty() {
                continue;
            }
            candidate_of[index] = Some(candidates.len());
            candidates.push(Candidate {
                request: index,
                gpu,
                joint_count,
                shapes,
            });
        }
        if !candidates.is_empty() {
            let wanted: Vec<Wanted> = candidates
                .iter()
                .map(|candidate| Wanted {
                    key: requests[candidate.request].mesh,
                    gpu: candidate.gpu,
                    shapes: &candidate.shapes,
                })
                .collect();
            self.arena.take_up((device, queue), &wanted);
        }
        // Each pair whose data the arena holds gets its corners and a job.
        let mut jobs: Vec<JobRaw> = Vec::with_capacity(candidates.len());
        let mut active: Vec<ActiveShape> = Vec::new();
        let mut posed_at: Vec<Option<u32>> = vec![None; candidates.len()];
        let (mut corners, mut sums, mut moves) = (0u64, 0u64, 0u64);
        for (at, candidate) in candidates.iter().enumerate() {
            let request = &requests[candidate.request];
            let version = (request.mesh, candidate.gpu.version);
            let Some(source) = self.arena.sources.get(&version) else {
                continue;
            };
            let firsts: Vec<u32> = candidate
                .shapes
                .iter()
                .map(|(shape, _)| source.shapes[*shape as usize])
                .collect();
            if firsts.contains(&NONE) {
                continue;
            }
            let shape_moves: u32 = candidate.shapes.iter().map(|(_, shape)| shape.count).sum();
            let offset = corners as u32;
            jobs.push(JobRaw {
                posed: offset,
                corners: candidate.gpu.vertex_count,
                sums: if candidate.shapes.is_empty() {
                    NONE
                } else {
                    sums as u32
                },
                joints: request.joints,
                joint_count: candidate.joint_count,
                active_first: active.len() as u32,
                active_count: candidate.shapes.len() as u32,
                moves: shape_moves,
                moves_start: moves as u32,
                vertices: source.vertices,
                skin: source.skin,
            });
            // Each shape's moves are where the arena took them up.
            active.extend(
                candidate
                    .shapes
                    .iter()
                    .zip(firsts)
                    .map(|((_, shape), first)| ActiveShape { first, ..*shape }),
            );
            if !candidate.shapes.is_empty() {
                sums += u64::from(candidate.gpu.vertex_count);
            }
            corners += u64::from(candidate.gpu.vertex_count);
            moves += u64::from(shape_moves);
            posed_at[at] = Some(offset);
            self.offsets.insert(
                (std::ptr::from_ref(request.skin).addr(), request.mesh),
                offset,
            );
        }
        self.corners = corners;
        self.moves = moves;
        if jobs.is_empty() {
            self.group = None;
        } else {
            self.upload((device, queue), (&jobs, &active), sums, joints);
        }
        let previous_bytes = if keep_previous {
            self.previous.size()
        } else {
            0
        };
        self.stats = PosingStats {
            items: jobs.len() as u32,
            corners,
            shapes: active.len() as u32,
            moves,
            bytes: self.posed.size()
                + previous_bytes
                + self.sums.size()
                + self.jobs.size()
                + self.active.size()
                + self.arena.bytes(),
        };
        candidate_of
            .iter()
            .map(|candidate| candidate.and_then(|candidate| posed_at[candidate]))
            .collect()
    }

    /// Writes this frame's lists, growing any buffer they don't fit (the
    /// posed buffer keeps last frame's corners for the copy), and binds
    /// them.
    fn upload(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        (jobs, active): (&[JobRaw], &[ActiveShape]),
        sums: u64,
        joints: &wgpu::Buffer,
    ) {
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let grow = |target: &mut wgpu::Buffer, label, bytes: u64, usage| {
            if bytes > target.size() {
                *target = buffer(device, label, bytes, usage);
            }
        };
        grow(
            &mut self.jobs,
            "posed items",
            size_of_val(jobs) as u64,
            storage,
        );
        grow(
            &mut self.active,
            "active shapes",
            size_of_val(active) as u64,
            storage,
        );
        // New sums are zeroed; `cs_pose` sets each back to 0 once read.
        grow(
            &mut self.sums,
            "shape sums",
            sums * SUMS_PER_CORNER * 4,
            wgpu::BufferUsages::STORAGE,
        );
        let posed_bytes = self.corners * POSED_STRIDE;
        if posed_bytes > self.posed.size() {
            let fresh = buffer(device, "posed corners", posed_bytes, posed_usage(device));
            let old = std::mem::replace(&mut self.posed, fresh);
            if self.copy_previous {
                self.retired = Some(old);
            }
            self.generation += 1;
        }
        if self.copy_previous && self.last_corners * POSED_STRIDE > self.previous.size() {
            self.previous = buffer(
                device,
                "last frame's posed corners",
                self.last_corners * POSED_STRIDE,
                posed_usage(device),
            );
            self.generation += 1;
        }
        let totals = TotalsRaw {
            jobs: jobs.len() as u32,
            corners: self.corners as u32,
            moves: self.moves as u32,
            stride: (size_of::<crate::mesh::Vertex>() / 4) as u32,
        };
        queue.write_buffer(&self.totals, 0, bytemuck::bytes_of(&totals));
        queue.write_buffer(&self.jobs, 0, bytemuck::cast_slice(jobs));
        if !active.is_empty() {
            queue.write_buffer(&self.active, 0, bytemuck::cast_slice(active));
        }
        let buffers = [
            &self.totals,
            &self.jobs,
            &self.active,
            joints,
            &self.sums,
            &self.posed,
            &self.arena.vertices,
            &self.arena.skin,
            &self.arena.moves,
        ];
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        self.group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("posing"),
            layout: &self.layout,
            entries: &entries,
        }));
    }

    /// Records this frame's posing: last frame's corners copied aside (if
    /// kept), new meshes' corners copied into the arena, then every item's
    /// shapes and every item's corners, one dispatch each. True if
    /// anything was posed.
    pub(super) fn record<'a>(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        mesh: impl Fn(MeshKey) -> Option<&'a GpuMesh>,
    ) -> bool {
        if self.copy_previous {
            let bytes = self.last_corners * POSED_STRIDE;
            let source = self.retired.as_ref().unwrap_or(&self.posed);
            if bytes <= source.size() && bytes <= self.previous.size() {
                encoder.copy_buffer_to_buffer(source, 0, &self.previous, 0, bytes);
            }
        }
        self.retired = None;
        for (key, first) in self.arena.copies.drain(..) {
            if let Some(gpu) = mesh(key) {
                let bytes = u64::from(gpu.vertex_count) * ARENA_SIZES[0];
                encoder.copy_buffer_to_buffer(
                    &gpu.vertices,
                    0,
                    &self.arena.vertices,
                    u64::from(first) * ARENA_SIZES[0],
                    bytes,
                );
            }
        }
        let Some(group) = self.group.as_ref().filter(|_| self.corners > 0) else {
            return false;
        };
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("posing"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, group, &[]);
        if self.moves > 0 {
            pass.set_pipeline(&self.shapes_pipeline);
            let (x, y) = rows(self.moves, SHAPE_GROUP);
            pass.dispatch_workgroups(x, y, 1);
        }
        pass.set_pipeline(&self.pose_pipeline);
        let (x, y) = rows(self.corners, POSE_GROUP);
        pass.dispatch_workgroups(x, y, 1);
        true
    }

    /// Reads back `count` posed corners from `first` (for tests): each
    /// one's position, normal and tangent.
    #[cfg(test)]
    pub(super) fn read(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        first: u32,
        count: u32,
    ) -> Vec<[Vec3; 3]> {
        let bytes = u64::from(count) * POSED_STRIDE;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("posed corners read back"),
            size: bytes.max(4),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.posed,
            u64::from(first) * POSED_STRIDE,
            &staging,
            0,
            bytes,
        );
        queue.submit([encoder.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU finished");
        let mapped = staging
            .slice(..)
            .get_mapped_range()
            .expect("the buffer is mapped");
        let words: Vec<u32> = bytemuck::cast_slice(&mapped).to_vec();
        words
            .chunks_exact(POSED_WORDS as usize)
            .map(|corner| {
                [
                    Vec3::new(
                        f32::from_bits(corner[0]),
                        f32::from_bits(corner[1]),
                        f32::from_bits(corner[2]),
                    ),
                    from_octahedral(corner[3]),
                    from_octahedral(corner[4]),
                ]
            })
            .collect()
    }
}

/// Must match `ZERO_DIRECTION` in posing.wgsl and shader.wgsl.
#[cfg(test)]
const ZERO_DIRECTION: u32 = 0x8000_8000;

/// A direction from its octahedral word, as `from_octahedral` in
/// shader.wgsl reads it.
#[cfg(test)]
fn from_octahedral(word: u32) -> Vec3 {
    if word == ZERO_DIRECTION {
        return Vec3::ZERO;
    }
    let snorm = |bits: u32| (f32::from((bits & 0xFFFF) as u16 as i16) / 32767.0).max(-1.0);
    let (x, y) = (snorm(word), snorm(word >> 16));
    let mut v = Vec3::new(x, y, 1.0 - x.abs() - y.abs());
    let t = (-v.z).max(0.0);
    v.x += if v.x >= 0.0 { -t } else { t };
    v.y += if v.y >= 0.0 { -t } else { t };
    v.normalize()
}

impl super::ScenePipeline {
    /// Poses this frame's animated items (see the module's notes): every
    /// (skin, mesh) pair of `uses` that a camera instance or a shadow
    /// view's caster draws, and, with `rays` (the camera and the ray
    /// tracing distance), every animated draw ray tracing will trace. The
    /// pending `skin` of each of `instances` and `view_casters` becomes
    /// its posed corners (or `[0, 0]`, drawn at rest). `joints` gets the
    /// joints of skins only ray tracing poses.
    pub(super) fn pose_frame(
        &mut self,
        draws: &[super::Draw<'_>],
        (instances, view_casters): (
            &mut [Vec<super::types::InstanceRaw>],
            &mut [super::lighting::ViewCasters],
        ),
        (joints, skins, uses): (
            &mut Vec<super::skinning::JointRaw>,
            &std::collections::HashMap<*const Skin, u32>,
            &[super::lighting::PoseUse],
        ),
        rays: Option<(glam::Vec3, f32)>,
    ) {
        // The pairs some view draws: every camera instance, and the casters
        // a shadow view kept.
        let mut used = vec![false; uses.len()];
        let all = instances.iter().flatten().chain(
            view_casters
                .iter()
                .flat_map(|view| view.instances.iter().flatten()),
        );
        for raw in all {
            if raw.skin[1] == PENDING {
                used[raw.skin[0] as usize] = true;
            }
        }
        let mut requests: Vec<PoseRequest> = Vec::new();
        let mut request_of = vec![None; uses.len()];
        for (index, pose) in uses.iter().enumerate() {
            let skin = draws[pose.draw as usize].item.skin.as_deref();
            let Some(skin) = skin.filter(|_| used[index]) else {
                continue;
            };
            request_of[index] = Some(requests.len());
            requests.push(PoseRequest {
                skin,
                mesh: pose.mesh,
                joints: pose.joints,
            });
        }
        // Ray tracing also traces what no view draws (behind the camera,
        // or casting no shadow), posed the same.
        if let Some((camera, reach)) = rays {
            // The pairs already asked for (made only while rays are traced).
            let mut asked: FxHashMap<(*const Skin, MeshKey), ()> = requests
                .iter()
                .map(|request| ((std::ptr::from_ref(request.skin), request.mesh), ()))
                .collect();
            let mut more_skins: FxHashMap<*const Skin, u32> = FxHashMap::default();
            for draw in draws {
                if draw.fade > 1.0 {
                    continue;
                }
                let Some(skin) = draw.item.skin.as_deref() else {
                    continue;
                };
                let mesh = draw.mesh();
                let key = (std::ptr::from_ref(skin), mesh);
                if asked.contains_key(&key) {
                    continue;
                }
                let gpu = super::scene::gpu_mesh_in(&self.meshes, &self.custom_meshes, mesh);
                let Some(gpu) = gpu.filter(|gpu| gpu.skin.is_some()) else {
                    continue;
                };
                if super::ray_tracing::within_reach(draw.item, gpu, camera, reach).is_none() {
                    continue;
                }
                let first = match skins.get(&key.0).or_else(|| more_skins.get(&key.0)) {
                    Some(&first) => first,
                    None => {
                        let first = joints.len() as u32;
                        joints.extend(super::skinning::JointRaw::of(skin));
                        more_skins.insert(key.0, first);
                        first
                    }
                };
                asked.insert(key, ());
                requests.push(PoseRequest {
                    skin,
                    mesh,
                    joints: first,
                });
            }
        }
        self.skinning
            .write_joints(&self.device, &self.queue, joints);
        let (meshes, custom_meshes) = (&self.meshes, &self.custom_meshes);
        let offsets = self.posing.prepare(
            (&self.device, &self.queue),
            &requests,
            |key| super::scene::gpu_mesh_in(meshes, custom_meshes, key),
            &self.skinning.joints,
            self.frame_targets.temporal.is_some(),
        );
        // Each pending instance and caster gets its pair's corners.
        let posed: Vec<Option<u32>> = request_of
            .iter()
            .map(|request| request.and_then(|request| offsets[request]))
            .collect();
        let all = instances.iter_mut().flatten().chain(
            view_casters
                .iter_mut()
                .flat_map(|view| view.instances.iter_mut().flatten()),
        );
        for raw in all {
            if raw.skin[1] == PENDING {
                raw.skin = match posed[raw.skin[0] as usize] {
                    Some(first) => [first, 1],
                    None => [0, 0],
                };
            }
        }
    }
}
