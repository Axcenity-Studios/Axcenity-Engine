//! Still items of the ray traced scene grouped into shared BLASes
//! (`docs/rendering/ray-tracing.md`).
//!
//! A meadow is a hundred thousand small plants, and as one TLAS instance
//! each it cost Overgrown Street about 25 ms of the frame's thread a frame
//! (the slots, the instances, then wgpu's encoding and submit, which walk
//! every instance). Small still items are grouped instead by the square of
//! ground they stand on ([`super::ray_sway::bundle_cell`]), each group one
//! BLAS whose geometries are its items' own meshes placed by a transform
//! (`BlasTriangleGeometry::transform_buffer`), and one TLAS instance. A
//! hit's slot is still its instance's first one plus the geometry hit, as
//! for bent plants, so every slot table reads the same.
//!
//! - Only items of at most [`GROUPED_TRIANGLES`] triangles: a group holds
//!   its own copy of its items' triangles (instancing doesn't), so the
//!   trees, whose triangles are most of a forest's, keep their instances.
//! - A group's BLAS is built when its items change (an item comes, goes,
//!   changes its level of detail or moves) and kept while they don't, up
//!   to [`GROUP_TRIANGLES_PER_FRAME`] triangles of groups built a frame,
//!   nearest first. A group waiting for its turn traces its items as
//!   instances of their own, so the picture never waits.

use std::hash::{Hash, Hasher};

use rustc_hash::{FxHashMap, FxHasher};

use glam::Mat4;

use super::{GpuMesh, MeshKey};

/// Items with at most this many triangles are grouped.
pub(super) const GROUPED_TRIANGLES: u32 = 256;

/// The most triangles built into groups' BLASes in one frame: about 2.6 ms
/// of the GPU on an RX 9070 XT (2.6 ms a million,
/// `docs/research/hardware-ray-tracing.md`).
pub(super) const GROUP_TRIANGLES_PER_FRAME: u64 = 1_000_000;

/// Fewer items than this are left as instances of their own: a BLAS for
/// two or three saves little.
pub(super) const MIN_GROUP: usize = 4;

/// A group no frame has used for this many frames is freed.
const FORGET_AFTER: u64 = 120;

/// Which group: whether its items are cut-outs (their triangles aren't
/// opaque to rays), their TLAS mask, and whether their meshes' indices are
/// 32 bits wide (what one TLAS instance's items must share), then the
/// square of ground they stand on.
pub(super) type GroupKey = (bool, u8, bool, [i32; 3]);

/// The bytes of one item's transform, the rows of a 3 by 4 matrix.
const TRANSFORM_BYTES: u64 = 48;

/// One item of a group: its mesh, whether it's a cut-out, and the rows of
/// its 3 by 4 transform.
pub(super) type Member<'a> = (&'a GpuMesh, bool, [f32; 12]);

struct Group {
    blas: wgpu::Blas,
    /// Each item's size, in geometry order.
    sizes: Vec<wgpu::BlasTriangleGeometrySizeDescriptor>,
    /// Each item's transform, in geometry order.
    transforms: wgpu::Buffer,
    /// Of the items it was made for ([`fingerprint`]).
    fingerprint: u64,
    last_used: u64,
    /// Traced this frame.
    used: bool,
    /// Built this frame: its build goes with the frame's.
    building: bool,
}

/// The ray traced scene's groups of still items.
#[derive(Default)]
pub(super) struct StillGroups {
    groups: FxHashMap<GroupKey, Group>,
}

/// What one item adds to its group's fingerprint: its mesh (and the
/// mesh's version), whether it's a cut-out, and where it stands.
pub(super) fn item_print(mesh: MeshKey, gpu: &GpuMesh, cut_out: bool, transform: &Mat4) -> u64 {
    let mut hasher = FxHasher::default();
    mesh.hash(&mut hasher);
    gpu.version.hash(&mut hasher);
    cut_out.hash(&mut hasher);
    let words: &[u64; 8] = bytemuck::cast_ref(transform.as_ref());
    words.hash(&mut hasher);
    hasher.finish()
}

/// A group's fingerprint so far (`print`) with one more item's
/// ([`item_print`]): its items' prints in order, so the same items in
/// another order are another group.
pub(super) fn fold_print(print: u64, item: u64) -> u64 {
    print.rotate_left(5).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ item
}

impl StillGroups {
    /// Starts a frame: no group is traced yet.
    pub(super) fn begin(&mut self) {
        for group in self.groups.values_mut() {
            group.used = false;
            group.building = false;
        }
    }

    /// Traces group `key` this frame as it was made, if it was made for
    /// `count` items whose fingerprint is `print`. Returns whether it was.
    pub(super) fn keep(
        &mut self,
        key: &GroupKey,
        (count, print): (usize, u64),
        frame: u64,
    ) -> bool {
        let Some(group) = self.groups.get_mut(key) else {
            return false;
        };
        if group.fingerprint != print || group.sizes.len() != count {
            return false;
        }
        group.used = true;
        group.last_used = frame;
        true
    }

    /// Makes group `key` for `members` (whose fingerprint is `print`) and
    /// builds it this frame, if `budget` (triangles still to build this
    /// frame) allows. Returns whether it's traced this frame; if not, its
    /// items are to be traced on their own.
    pub(super) fn build(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        key: GroupKey,
        (members, print): (&[Member<'_>], u64),
        budget: &mut u64,
        frame: u64,
    ) -> bool {
        let triangles: u64 = members
            .iter()
            .map(|(gpu, _, _)| u64::from(gpu.index_count / 3))
            .sum();
        if triangles > *budget {
            return false;
        }
        *budget -= triangles;
        let sizes: Vec<_> = members
            .iter()
            .map(|(gpu, cut_out, _)| super::ray_tracing::size_of_mesh(gpu, *cut_out))
            .collect();
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("still group BLAS"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE
                    | wgpu::AccelerationStructureFlags::USE_TRANSFORM,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: sizes.clone(),
            },
        );
        let rows: Vec<[f32; 12]> = members.iter().map(|(_, _, rows)| *rows).collect();
        let transforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("still group transforms"),
            size: members.len() as u64 * TRANSFORM_BYTES,
            usage: super::ray_tracing::blas_input(device) | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&transforms, 0, bytemuck::cast_slice(&rows));
        self.groups.insert(
            key,
            Group {
                blas,
                sizes,
                transforms,
                fingerprint: print,
                last_used: frame,
                used: true,
                building: true,
            },
        );
        true
    }

    /// The BLAS of group `key`, if it's traced this frame.
    pub(super) fn blas(&self, key: &GroupKey) -> Option<&wgpu::Blas> {
        let group = self.groups.get(key)?;
        group.used.then_some(&group.blas)
    }

    /// Whether group `key` is built this frame.
    pub(super) fn building(&self, key: &GroupKey) -> bool {
        self.groups.get(key).is_some_and(|group| group.building)
    }

    /// Whether any group was built this frame.
    pub(super) fn any_building(&self) -> bool {
        self.groups.values().any(|group| group.building)
    }

    /// What builds group `key`'s BLAS this frame from its items' meshes
    /// (`meshes`, in its geometries' order), or `None` if it's kept from an
    /// earlier frame.
    pub(super) fn entry<'a>(
        &'a self,
        key: &GroupKey,
        meshes: &[&'a GpuMesh],
    ) -> Option<wgpu::BlasBuildEntry<'a>> {
        let group = self.groups.get(key)?;
        // A geometry left out would move the ones after it to other slots.
        if !group.building || group.sizes.len() != meshes.len() {
            return None;
        }
        let geometries = group
            .sizes
            .iter()
            .zip(meshes)
            .enumerate()
            .map(|(at, (size, gpu))| wgpu::BlasTriangleGeometry {
                size,
                // `Vertex::position` comes first in each corner.
                vertex_buffer: &gpu.vertices,
                first_vertex: 0,
                vertex_stride: size_of::<crate::mesh::Vertex>() as u64,
                index_buffer: Some(&gpu.indices),
                first_index: Some(0),
                transform_buffer: Some(&group.transforms),
                transform_buffer_offset: Some(at as u64 * TRANSFORM_BYTES),
            })
            .collect();
        Some(wgpu::BlasBuildEntry {
            blas: &group.blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(geometries),
        })
    }

    /// Frees the groups no frame has traced for a while.
    pub(super) fn forget_unused(&mut self, frame: u64) {
        self.groups
            .retain(|_, group| frame.saturating_sub(group.last_used) < FORGET_AFTER);
    }

    /// How many groups are traced this frame, and how many were built.
    pub(super) fn counts(&self) -> (u32, u32) {
        let used = self.groups.values().filter(|group| group.used).count() as u32;
        let built = self.groups.values().filter(|group| group.building).count() as u32;
        (used, built)
    }
}
