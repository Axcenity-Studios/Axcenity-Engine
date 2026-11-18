//! Animated meshes on the GPU: what they're uploaded with.
//!
//! An animated mesh's corners are uploaded once, as they are in its file,
//! and each corner's joints and weights ([`MeshSource::skin_joints`]) and
//! its blend shapes (morph targets, facial animation F2, identity shapes,
//! ADR-0041) are kept ready for the posing arena (`posing.rs`), which takes
//! them up the first time the mesh is posed. A mesh's shapes are kept shape
//! by shape, each a range of moves ([`ShapeRange`]: only the corners it
//! moves), so the pass can walk the moves of the shapes an item uses and
//! skip the rest.
//!
//! Each frame the joints of every [`Skin`] posed go into one shared buffer
//! ([`Skinning::joints`]), and `posing.rs` morphs and skins each (skin,
//! mesh) pair once into the frame's posed buffer, which every pass's
//! vertex shader reads (CO8). Items posed differently still share one copy
//! of the mesh, and batch together.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::draw_list::{MeshSource, Skin};

use super::Bounds;
use super::posing::ShapeRange;

/// One joint as the GPU reads it. Must match `Joint` in posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct JointRaw {
    pub(super) position: [[f32; 4]; 4],
    /// A `mat3x3` in a storage buffer: each column is 16 bytes.
    pub(super) normal: [[f32; 4]; 3],
}

impl JointRaw {
    /// Each joint of `skin`. Its shape weights reach the posing pass as
    /// each item's list of active shapes instead (`posing.rs`).
    pub(super) fn of(skin: &Skin) -> impl Iterator<Item = Self> + '_ {
        skin.joints()
            .iter()
            .zip(skin.normals())
            .map(|(joint, normal)| Self {
                position: joint.to_cols_array_2d(),
                normal: [normal.x_axis, normal.y_axis, normal.z_axis].map(|column| {
                    let [x, y, z] = column.to_array();
                    [x, y, z, 0.0]
                }),
            })
    }
}

/// One corner's move by one shape at weight 1. Must match `Move` in
/// posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub(super) struct MoveRaw {
    position: [f32; 3],
    corner: u32,
    normal: [f32; 3],
    _pad: u32,
}

/// One corner's joints and weights. Must match `SkinCorner` in
/// posing.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub(super) struct SkinCorner {
    joints: [u32; 4],
    weights: [f32; 4],
}

/// An animated mesh's joints and weights, and its blend shapes, on the GPU.
pub(super) struct GpuSkin {
    /// Its corners' joints and weights (none for a mesh with blend shapes
    /// and no joints), and its shapes' moves, shape by shape: what the
    /// posing arena takes up (`posing.rs`). Kept here so the arena can be
    /// made again.
    pub(super) corners: Vec<SkinCorner>,
    pub(super) moves: Vec<MoveRaw>,
    /// Its blend shapes, in the mesh's order: where each one's moves are
    /// and which weight drives it.
    pub(super) shapes: Vec<ShapeRange>,
    /// Whether the mesh has joints (and not only blend shapes).
    pub(super) jointed: bool,
    /// The box of the corners weighted to each joint, by joint, `None` for
    /// a joint no corner is weighted to; and the box of the corners
    /// weighted to nothing, which never move.
    joints: Vec<Option<Bounds>>,
    still: Option<Bounds>,
}

impl GpuSkin {
    /// The box `skin` moves the corners into. A corner ends up between the
    /// places its joints move it to (its weights add up to 1), so it's
    /// inside the box around each joint's corners as that joint moves them.
    pub(super) fn bounds(&self, skin: &Skin) -> Bounds {
        let moved = self
            .joints
            .iter()
            .enumerate()
            .filter_map(|(joint, bounds)| {
                let bounds = (*bounds)?;
                // A joint past the skin's is skipped, as in the shader.
                Some((bounds, *skin.joints().get(joint)?))
            });
        let corners = moved
            .flat_map(|(bounds, joint)| {
                corners(bounds).map(|corner| joint.transform_point3(corner))
            })
            .chain(self.still.into_iter().flat_map(corners));
        Bounds::around(corners)
    }
}

fn corners(Bounds { min, max }: Bounds) -> [Vec3; 8] {
    [0, 1, 2, 3, 4, 5, 6, 7].map(|bits| {
        Vec3::new(
            if bits & 1 == 0 { min.x } else { max.x },
            if bits & 2 == 0 { min.y } else { max.y },
            if bits & 4 == 0 { min.z } else { max.z },
        )
    })
}

/// What animated meshes are uploaded with, and the frame's joints.
#[derive(Clone)]
pub(super) struct Skinning {
    /// Group 2 of the scene's pipelines: empty since CO8 (each pass reads
    /// its posed corners through the frame's binding 9), kept so every
    /// pipeline's groups stay where they were.
    pub(super) layout: wgpu::BindGroupLayout,
    pub(super) none: wgpu::BindGroup,
    /// Group 1 of the shadow passes, which bind no material.
    pub(super) empty_layout: wgpu::BindGroupLayout,
    pub(super) empty: wgpu::BindGroup,
    /// Every posed skin's joints this frame, read by the posing pass.
    pub(super) joints: wgpu::Buffer,
}

/// Binding 9 of group 0 (the frame's and the shadow passes'): the frame's
/// posed corners (`posing.rs`), read by the vertex shaders.
pub(super) const POSED_BINDING: u32 = 9;

pub(super) fn posed_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: POSED_BINDING,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: storage(),
        count: None,
    }
}

fn storage() -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}

impl Skinning {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let empty_group = |label| {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[],
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[],
            });
            (layout, group)
        };
        let (layout, none) = empty_group("skin");
        let (empty_layout, empty) = empty_group("nothing");
        Self {
            layout,
            none,
            empty_layout,
            empty,
            joints: joint_buffer(device, 64),
        }
    }

    /// Writes this frame's `joints`, first making the buffer bigger if
    /// they don't fit.
    pub(super) fn write_joints(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        joints: &[JointRaw],
    ) {
        let needed = size_of_val(joints) as u64;
        if needed > self.joints.size() {
            let capacity = joints.len().next_power_of_two();
            self.joints = joint_buffer(device, capacity);
        }
        if !joints.is_empty() {
            queue.write_buffer(&self.joints, 0, bytemuck::cast_slice(joints));
        }
    }

    /// Gets `source`'s joints and weights and its blend shapes ready for
    /// the posing arena, or `None` if it has neither, or (with a warning)
    /// if its joints and weights don't fit its corners: it's then drawn as
    /// it is, though still morphed.
    pub(super) fn upload(&self, source: &dyn MeshSource) -> Option<GpuSkin> {
        let positions = source.positions();
        let (joints, weights) = (source.skin_joints(), source.skin_weights());
        let mut jointed = !(joints.is_empty() && weights.is_empty());
        if jointed && (joints.len() != positions.len() || weights.len() != positions.len()) {
            log::warn!(
                "mesh #{} has joints or weights that don't fit its corners, so it isn't animated",
                source.key()
            );
            jointed = false;
        }
        let morphs = morph_moves(source);
        if !jointed && morphs.is_none() {
            return None;
        }
        let corners: Vec<SkinCorner> = match jointed {
            true => joints
                .iter()
                .zip(weights)
                .map(|(joints, weights)| SkinCorner {
                    joints: joints.map(u32::from),
                    weights: *weights,
                })
                .collect(),
            false => Vec::new(),
        };
        // How far each corner's shapes can move it, all at weight 1.
        let mut reach = vec![0.0f32; positions.len()];
        if let Some((_, moves)) = &morphs {
            for one in moves {
                reach[one.corner as usize] += Vec3::from(one.position).length();
            }
        }
        // Each joint's corners' box, and the box of those that never move,
        // each grown by how far their shapes can move them.
        let mut joint_bounds: Vec<Option<Bounds>> = Vec::new();
        let mut still: Option<Bounds> = None;
        let grow = |bounds: &mut Option<Bounds>, point: Vec3, reach: f32| {
            let (low, high) = (point - Vec3::splat(reach), point + Vec3::splat(reach));
            *bounds = Some(match *bounds {
                Some(Bounds { min, max }) => Bounds {
                    min: min.min(low),
                    max: max.max(high),
                },
                None => Bounds {
                    min: low,
                    max: high,
                },
            });
        };
        for (index, position) in positions.iter().enumerate() {
            let point = Vec3::from(*position);
            let reach = reach[index];
            let mut weighted = false;
            if jointed {
                let corner = corners[index];
                for (&joint, &weight) in corner.joints.iter().zip(&corner.weights) {
                    if weight <= 0.0 {
                        continue;
                    }
                    let joint = joint as usize;
                    if joint >= joint_bounds.len() {
                        joint_bounds.resize(joint + 1, None);
                    }
                    grow(&mut joint_bounds[joint], point, reach);
                    weighted = true;
                }
            }
            if !weighted {
                grow(&mut still, point, reach);
            }
        }
        let (shapes, moves) = morphs.unwrap_or_default();
        Some(GpuSkin {
            corners,
            moves,
            shapes,
            jointed,
            joints: joint_bounds,
            still,
        })
    }
}

/// `source`'s blend shapes as each shape's range of moves and the moves,
/// shape after shape, or `None` if it has none (or they don't fit its
/// corners).
fn morph_moves(source: &dyn MeshSource) -> Option<(Vec<ShapeRange>, Vec<MoveRaw>)> {
    let corners = source.positions().len();
    let mut shapes = Vec::with_capacity(source.morph_count());
    let mut moves: Vec<MoveRaw> = Vec::new();
    for index in 0..source.morph_count() {
        let Some(morph) = source.morph(index) else {
            continue;
        };
        let first = moves.len() as u32;
        let changes = morph.corners.iter().zip(morph.positions).zip(
            morph
                .normals
                .iter()
                .copied()
                .chain(std::iter::repeat([0.0; 3])),
        );
        for ((&corner, position), normal) in changes {
            if corner as usize >= corners {
                log::warn!(
                    "mesh #{} has a blend shape moving a corner it hasn't got, so it isn't morphed",
                    source.key()
                );
                return None;
            }
            moves.push(MoveRaw {
                position: *position,
                corner,
                normal,
                _pad: 0,
            });
        }
        shapes.push(ShapeRange {
            first,
            count: moves.len() as u32 - first,
            target: u32::from(morph.target),
        });
    }
    if moves.is_empty() {
        return None;
    }
    Some((shapes, moves))
}

fn joint_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("joints"),
        size: (capacity.max(1) * size_of::<JointRaw>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
