//! How long the GPU spends on a frame, and on each part of it, for the
//! editor's profiler and the benchmarks.
//!
//! Only devices opened with `Features::TIMESTAMP_QUERY` can be timed; the
//! player never asks for it, so it never times anything. An empty compute
//! pass writes a timestamp before each [`FramePass`] and one after the
//! last, so each part's time runs from its stamp to the next. (A part such
//! as the shadows or bloom is many render passes, so stamping between parts
//! times them whole.) The results are copied into a buffer that's read once
//! the GPU is done with it, a frame or two later, so measuring never makes
//! the CPU wait for the GPU.
//!
//! Within the depth prepass and the scene pass, which are one render pass
//! each, the time is also taken by what is drawn ([`DrawnPart`]): the
//! terrain, the items scattered over it, the other items, the sky. That
//! needs `Features::TIMESTAMP_QUERY_INSIDE_PASSES` too (every desktop
//! GPU), since the stamps are written between draws: one where the part
//! being drawn changes ([`GpuTimer::part`]). Solid items are sorted by
//! distance band and then by part, so that is a few dozen stamps a pass.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// The parts of a frame the GPU is timed on, in the order they're drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FramePass {
    /// Animated characters morphed and skinned once (CO8), before every
    /// pass that draws or traces them.
    CharacterPosing,
    /// Hardware ray tracing's acceleration structures built, while
    /// something traces rays.
    AccelerationStructures,
    /// The sun's cascades and the spot lights' shadow maps.
    Shadows,
    /// Solid items' depth and normals, while ambient occlusion is on.
    Prepass,
    /// Ambient occlusion and its blur.
    AmbientOcclusion,
    /// Ray traced sun shadows' rays and smoothing, while they're on.
    RayTracedShadows,
    /// Ray traced GI's batch of light probes, while it's on.
    RayTracedGi,
    /// Ray traced reflections' rays, while they're on.
    RayTracedReflections,
    /// The volumetric fog's grid, while it's on (its light is added to the
    /// scene as part of `Scene`).
    VolumetricFog,
    /// GPU particles moved, sorted and written as sprites.
    Particles,
    /// The sky's clouds traced at half size, while they are (their shapes'
    /// texture too, on the frames that draw it).
    Clouds,
    /// The sky, items, terrain and sprites, with MSAA resolved.
    Scene,
    /// Bloom, tone mapping and FXAA.
    PostProcessing,
    /// Turned text and the UI overlay.
    Ui,
}

impl FramePass {
    pub const COUNT: usize = 14;
    pub const ALL: [FramePass; Self::COUNT] = [
        FramePass::CharacterPosing,
        FramePass::AccelerationStructures,
        FramePass::Shadows,
        FramePass::Prepass,
        FramePass::AmbientOcclusion,
        FramePass::RayTracedShadows,
        FramePass::RayTracedGi,
        FramePass::RayTracedReflections,
        FramePass::VolumetricFog,
        FramePass::Particles,
        FramePass::Clouds,
        FramePass::Scene,
        FramePass::PostProcessing,
        FramePass::Ui,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FramePass::CharacterPosing => "Character posing",
            FramePass::AccelerationStructures => "Acceleration structures",
            FramePass::Shadows => "Shadows",
            FramePass::Prepass => "Depth prepass",
            FramePass::AmbientOcclusion => "Ambient occlusion",
            FramePass::RayTracedShadows => "Ray traced shadows",
            FramePass::RayTracedGi => "Ray traced GI",
            FramePass::RayTracedReflections => "Ray traced reflections",
            FramePass::VolumetricFog => "Volumetric fog",
            FramePass::Particles => "GPU particles",
            FramePass::Clouds => "Clouds",
            FramePass::Scene => "Scene",
            FramePass::PostProcessing => "Post-processing",
            FramePass::Ui => "UI",
        }
    }
}

/// What is drawn, within the depth prepass and within the scene pass.
/// Solid items are one of the first three, drawn in this order within each
/// distance band: the terrain last, so what stands on it hides it first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrawnPart {
    /// Solid items that are neither scattered nor terrain: models, parts,
    /// characters.
    Items,
    /// Solid items with a [`crate::DrawItem::cull_distance`]: the plants,
    /// stones and litter painted over a terrain.
    Scattered,
    /// Terrain.
    Terrain,
    /// The cut-out surfaces' depth, drawn before the scene is lit (scene
    /// pass only).
    CutOutDepth,
    /// The sky, with its clouds (scene pass only).
    Sky,
    /// Projected decals, the grid and screen-space outlines, over the
    /// solid scene (scene pass only).
    Decals,
    /// See-through items and water (scene pass only).
    SeeThrough,
    /// Particles and other sprites (scene pass only).
    Sprites,
}

impl DrawnPart {
    pub const COUNT: usize = 8;
    pub const ALL: [DrawnPart; Self::COUNT] = [
        DrawnPart::Items,
        DrawnPart::Scattered,
        DrawnPart::Terrain,
        DrawnPart::CutOutDepth,
        DrawnPart::Sky,
        DrawnPart::Decals,
        DrawnPart::SeeThrough,
        DrawnPart::Sprites,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DrawnPart::Items => "items",
            DrawnPart::Scattered => "scattered items",
            DrawnPart::Terrain => "terrain",
            DrawnPart::CutOutDepth => "cut-outs' depth",
            DrawnPart::Sky => "sky",
            DrawnPart::Decals => "decals, grid and outlines",
            DrawnPart::SeeThrough => "see-through items",
            DrawnPart::Sprites => "sprites",
        }
    }
}

/// The passes timed by what is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartPass {
    Prepass,
    Scene,
}

impl PartPass {
    pub const COUNT: usize = 2;
    pub const ALL: [PartPass; Self::COUNT] = [PartPass::Prepass, PartPass::Scene];

    pub fn label(self) -> &'static str {
        match self {
            PartPass::Prepass => "Depth prepass",
            PartPass::Scene => "Scene",
        }
    }
}

/// `part` within `pass`, by name, for a table of both.
pub fn part_label(pass: PartPass, part: DrawnPart) -> &'static str {
    match (pass, part) {
        (PartPass::Prepass, DrawnPart::Items) => "Depth prepass: items",
        (PartPass::Prepass, DrawnPart::Scattered) => "Depth prepass: scattered items",
        (PartPass::Prepass, DrawnPart::Terrain) => "Depth prepass: terrain",
        (PartPass::Prepass, DrawnPart::CutOutDepth) => "Depth prepass: cut-outs' depth",
        (PartPass::Prepass, DrawnPart::Sky) => "Depth prepass: sky",
        (PartPass::Prepass, DrawnPart::Decals) => "Depth prepass: decals, grid and outlines",
        (PartPass::Prepass, DrawnPart::SeeThrough) => "Depth prepass: see-through items",
        (PartPass::Prepass, DrawnPart::Sprites) => "Depth prepass: sprites",
        (PartPass::Scene, DrawnPart::Items) => "Scene: items",
        (PartPass::Scene, DrawnPart::Scattered) => "Scene: scattered items",
        (PartPass::Scene, DrawnPart::Terrain) => "Scene: terrain",
        (PartPass::Scene, DrawnPart::CutOutDepth) => "Scene: cut-outs' depth",
        (PartPass::Scene, DrawnPart::Sky) => "Scene: sky",
        (PartPass::Scene, DrawnPart::Decals) => "Scene: decals, grid and outlines",
        (PartPass::Scene, DrawnPart::SeeThrough) => "Scene: see-through items",
        (PartPass::Scene, DrawnPart::Sprites) => "Scene: sprites",
    }
}

/// Each timed pass's time by what was drawn: `None` for what it didn't
/// draw.
pub type PartTimes = [[Option<Duration>; DrawnPart::COUNT]; PartPass::COUNT];

/// How many stamps a frame's parts can take: one each time the part being
/// drawn changes, and one where a pass's last part ends. A frame that
/// needs more gives no part times.
const PART_STAMPS: u32 = 512;

/// The sun's cascades, timed within the Shadows pass.
pub(super) const CASCADES: usize = super::types::SHADOW_CASCADES;

/// A stamp before each pass and one after the last, then one before each
/// cascade and one after the last cascade.
const STAMPS: u32 = FramePass::COUNT as u32 + 1 + CASCADES as u32 + 1;
/// The stamp after the last pass.
const FRAME_END: u32 = FramePass::COUNT as u32;
/// The first cascade's stamp.
const CASCADE_STAMPS: u32 = FramePass::COUNT as u32 + 1;
const RESULT_BYTES: u64 = STAMPS as u64 * size_of::<u64>() as u64;
const PART_BYTES: u64 = PART_STAMPS as u64 * size_of::<u64>() as u64;
/// Where the stamps between draws start in the buffers: queries resolve
/// at a multiple of 256 bytes.
const PART_OFFSET: u64 = RESULT_BYTES.next_multiple_of(wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT);

/// The stamps written between draws: what each one starts (`None` where
/// it only ends the part before it).
struct PartStamps {
    queries: wgpu::QuerySet,
    /// This frame's stamps so far, in the order written.
    written: RefCell<Vec<Option<(PartPass, DrawnPart)>>>,
    /// Whether this frame asked for more stamps than there are.
    overflowed: Cell<bool>,
}

/// What became of the last request to read the results.
const PENDING: u8 = 0;
const MAPPED: u8 = 1;
const FAILED: u8 = 2;

/// A frame whose stamps are on their way back: which passes and cascades
/// it drew, and what each stamp between draws started.
struct Waiting {
    drawn: [bool; FramePass::COUNT],
    cascades: [bool; CASCADES],
    parts: Vec<Option<(PartPass, DrawnPart)>>,
}

pub(super) struct GpuTimer {
    queries: wgpu::QuerySet,
    /// The stamps between draws, where the device can write them.
    parts: Option<PartStamps>,
    /// Where the queries are resolved on the GPU.
    resolve: wgpu::Buffer,
    /// A copy of `resolve` the CPU can read.
    readback: wgpu::Buffer,
    /// Nanoseconds per timestamp tick.
    period: f64,
    /// While waiting for `readback` to be read: which passes that frame
    /// drew. No new frame is timed meanwhile, since the buffer is in use.
    waiting: Option<Waiting>,
    /// [`PENDING`], [`MAPPED`] or [`FAILED`]: set by the map callback, on
    /// whichever thread polls the device.
    mapped: Arc<AtomicU8>,
    /// The last frame measured, whole.
    last: Option<Duration>,
    /// Its passes, `None` for any it didn't draw.
    last_passes: [Option<Duration>; FramePass::COUNT],
    /// Its sun cascades, `None` for any it didn't draw (off, or kept from
    /// an earlier frame while the far cascades take turns).
    last_cascades: [Option<Duration>; CASCADES],
    /// Its prepass and scene pass by what was drawn.
    last_parts: PartTimes,
    /// How many measurements have been read, so a reader can tell a new
    /// one from the last one again.
    readings: u64,
}

impl GpuTimer {
    /// A timer, or `None` if `device` can't take timestamps.
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: STAMPS,
        });
        // Stamps between draws, where the device has them (set
        // `AXCENITY_NO_PART_TIMES` to time a frame without them).
        let parts = (device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES)
            && std::env::var_os("AXCENITY_NO_PART_TIMES").is_none())
        .then(|| PartStamps {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("part timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: PART_STAMPS,
            }),
            written: RefCell::new(Vec::with_capacity(PART_STAMPS as usize)),
            overflowed: Cell::new(false),
        });
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: PART_OFFSET + PART_BYTES,
                usage,
                mapped_at_creation: false,
            })
        };
        Some(Self {
            queries,
            parts,
            resolve: buffer(
                "frame timestamps resolve",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "frame timestamps readback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            period: f64::from(queue.get_timestamp_period()),
            waiting: None,
            mapped: Arc::new(AtomicU8::new(PENDING)),
            last: None,
            last_passes: [None; FramePass::COUNT],
            last_cascades: [None; CASCADES],
            last_parts: PartTimes::default(),
            readings: 0,
        })
    }

    /// The GPU time of the last frame measured.
    pub(super) fn last(&self) -> Option<Duration> {
        self.last
    }

    /// The GPU time of each pass of the last frame measured, in
    /// [`FramePass::ALL`]'s order: `None` for a pass it didn't draw.
    pub(super) fn last_passes(&self) -> [Option<Duration>; FramePass::COUNT] {
        self.last_passes
    }

    /// The GPU time of each sun cascade of the last frame measured.
    pub(super) fn last_cascades(&self) -> [Option<Duration>; CASCADES] {
        self.last_cascades
    }

    /// The last frame measured's prepass and scene pass by what was drawn.
    pub(super) fn last_parts(&self) -> PartTimes {
        self.last_parts
    }

    /// How many measurements have been read so far.
    pub(super) fn readings(&self) -> u64 {
        self.readings
    }

    /// Reads a finished measurement, if one is ready. Returns true if this
    /// frame can be timed (call [`Self::mark`] before each pass, then
    /// [`Self::end`]).
    pub(super) fn collect(&mut self, device: &wgpu::Device) -> bool {
        let Some(Waiting {
            drawn,
            cascades,
            parts,
        }) = self.waiting.take()
        else {
            return true;
        };
        // Runs the map callback if the GPU has finished, without waiting.
        let _ = device.poll(wgpu::PollType::Poll);
        match self.mapped.swap(PENDING, Ordering::AcqRel) {
            PENDING => {
                self.waiting = Some(Waiting {
                    drawn,
                    cascades,
                    parts,
                });
                return false;
            }
            MAPPED => {
                let bytes = self
                    .readback
                    .slice(..)
                    .get_mapped_range()
                    .expect("the buffer is mapped");
                let stamps: [u64; STAMPS as usize] =
                    bytemuck::pod_read_unaligned(&bytes[..RESULT_BYTES as usize]);
                let part_stamps: Vec<u64> = bytes[PART_OFFSET as usize..]
                    .chunks_exact(size_of::<u64>())
                    .take(parts.len())
                    .map(bytemuck::pod_read_unaligned)
                    .collect();
                drop(bytes);
                self.readback.unmap();
                let between = |start: u64, end: u64| {
                    // A clock that went backwards (some drivers, across
                    // power states) gives no reading rather than a wrong one.
                    (end >= start)
                        .then(|| Duration::from_nanos(((end - start) as f64 * self.period) as u64))
                };
                if let Some(whole) = between(stamps[0], stamps[FRAME_END as usize]) {
                    self.last = Some(whole);
                    self.readings += 1;
                    for (index, time) in self.last_passes.iter_mut().enumerate() {
                        *time = drawn[index]
                            .then(|| between(stamps[index], stamps[index + 1]))
                            .flatten();
                    }
                    // Each drawn cascade runs to the next drawn one's stamp,
                    // or to the stamp after the last.
                    let stamp = |cascade: usize| stamps[CASCADE_STAMPS as usize + cascade];
                    for index in 0..CASCADES {
                        let end = (index + 1..CASCADES)
                            .find(|&next| cascades[next])
                            .unwrap_or(CASCADES);
                        self.last_cascades[index] = cascades[index]
                            .then(|| between(stamp(index), stamp(end)))
                            .flatten();
                    }
                    // Each stamp between draws starts a part that runs to
                    // the next stamp; a part drawn in several runs (once a
                    // distance band) is their sum.
                    self.last_parts = PartTimes::default();
                    for (index, started) in parts.iter().enumerate() {
                        let (Some((pass, part)), Some(&end)) =
                            (*started, part_stamps.get(index + 1))
                        else {
                            continue;
                        };
                        if let Some(time) = between(part_stamps[index], end) {
                            *self.last_parts[pass as usize][part as usize]
                                .get_or_insert(Duration::ZERO) += time;
                        }
                    }
                }
            }
            // The device was lost; the next frame tries again.
            _ => {}
        }
        true
    }

    /// Writes the timestamp before `pass`.
    pub(super) fn mark(&self, encoder: &mut wgpu::CommandEncoder, pass: FramePass) {
        self.stamp(encoder, pass as u32);
    }

    /// Writes the timestamp before sun cascade `cascade`, or after the last
    /// cascade for `CASCADES`.
    pub(super) fn mark_cascade(&self, encoder: &mut wgpu::CommandEncoder, cascade: usize) {
        self.stamp(encoder, CASCADE_STAMPS + cascade as u32);
    }

    /// Writes a timestamp between `pass`'s draws: `started` is what the
    /// draws after it are (within `within`), or `None` where the part
    /// before it ends and nothing timed follows. Nothing on a device that
    /// can't stamp inside a pass.
    pub(super) fn part(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        within: PartPass,
        started: Option<DrawnPart>,
    ) {
        let Some(parts) = &self.parts else {
            return;
        };
        let mut written = parts.written.borrow_mut();
        if written.len() >= PART_STAMPS as usize {
            parts.overflowed.set(true);
            return;
        }
        pass.write_timestamp(&parts.queries, written.len() as u32);
        written.push(started.map(|part| (within, part)));
    }

    /// Writes the timestamp after the last pass, and copies them all where
    /// the CPU can read them.
    pub(super) fn end(&self, encoder: &mut wgpu::CommandEncoder) {
        self.stamp(encoder, FRAME_END);
        encoder.resolve_query_set(&self.queries, 0..STAMPS, &self.resolve, 0);
        if let Some(parts) = &self.parts {
            let written = parts.written.borrow().len() as u32;
            if written > 0 {
                encoder.resolve_query_set(&parts.queries, 0..written, &self.resolve, PART_OFFSET);
            }
        }
        encoder.copy_buffer_to_buffer(
            &self.resolve,
            0,
            &self.readback,
            0,
            PART_OFFSET + PART_BYTES,
        );
    }

    /// Asks to read the timestamps once the frame just submitted is done.
    /// `drawn` says which of its passes drew anything.
    pub(super) fn submitted(
        &mut self,
        drawn: [bool; FramePass::COUNT],
        cascades: [bool; CASCADES],
    ) {
        let mapped = self.mapped.clone();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let state = if result.is_ok() { MAPPED } else { FAILED };
                mapped.store(state, Ordering::Release);
            });
        // A frame that ran out of stamps gives no part times: some of its
        // parts would be missing from their sums.
        let parts = self.parts.as_ref().map_or_else(Vec::new, |parts| {
            let written = std::mem::take(&mut *parts.written.borrow_mut());
            if parts.overflowed.replace(false) {
                Vec::new()
            } else {
                written
            }
        });
        self.waiting = Some(Waiting {
            drawn,
            cascades,
            parts,
        });
    }

    /// An empty compute pass that only writes timestamp `index`.
    fn stamp(&self, encoder: &mut wgpu::CommandEncoder, index: u32) {
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("frame timestamp"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &self.queries,
                beginning_of_pass_write_index: Some(index),
                end_of_pass_write_index: None,
            }),
        });
    }
}
