//! Screen-space reflections (`docs/rendering/reflections.md`): a compute
//! pass (`screen_reflections.wgsl`) marches one ray along the mirrored view
//! from each glossy solid pixel of the prepass (its roughness, below
//! [`ROUGHNESS_CUTOFF`], kept in the prepass normal's w) across the screen
//! through the prepass's depth, and reads what it hits from last frame's
//! picture (the scene's color target, which still holds it before this
//! frame's scene pass draws over it), where that point was last frame. What
//! it finds goes in the same buffer ray traced reflections use, bound in
//! the frame's place 18, so the scene's `shade_surface` (lighting.wgsl's
//! `traced_reflected`) puts it over the reflection probes and the sky, and
//! shades the sky's reflection where the shadow maps say something stands
//! over the way it leaves (`sky_open_along`). Ray traced reflections, when
//! drawn, take its place.
//!
//! Exists only while the setting is on and nothing traces reflections. A
//! frame whose last frame wasn't drawn into the same targets (the first,
//! or after they're made again) clears the buffer in place of marching:
//! it reflects nothing on screen, but the sky's shading is already on.

use std::num::NonZeroU64;

use glam::{Mat4, Vec3};

use crate::graphics::EffectiveGraphics;

use super::types::OcclusionTargets;

/// Surfaces at least this rough aren't marched: their reflection is blurred
/// enough for the probes and the sky to stand in for it.
pub(super) const ROUGHNESS_CUTOFF: f32 = 0.45;

/// The farthest a ray is marched, in studs.
const REACH: f32 = 400.0;

/// Must match `ScreenReflections` in screen_reflections.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct ScreenReflectionUniform {
    clip_to_world: [[f32; 4]; 4],
    world_to_clip: [[f32; 4]; 4],
    previous: [[f32; 4]; 4],
    view_to_world: [[f32; 4]; 4],
    camera: [f32; 4],
    forward: [f32; 4],
    /// The traced size, how many scene pixels a traced one stands for each
    /// way, the frame's number.
    size: [u32; 4],
    /// The roughness cutoff, how far the rays reach, the projection's
    /// depth terms.
    settings: [f32; 4],
}

/// Bytes a traced pixel takes: rgb and its weight as half floats (as ray
/// traced reflections').
const PIXEL_BYTES: u64 = 8;

/// The pass's pipeline and buffers.
pub(super) struct ScreenReflections {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    /// What the rays found, bound in the frame's place 18.
    pub(super) buffer: wgpu::Buffer,
    /// The traced size it holds.
    size: (u32, u32),
    /// Last frame's camera (without TAA's jitter), while last frame was
    /// drawn into the targets this frame reads.
    pub(super) previous: Option<Mat4>,
}

impl ScreenReflections {
    /// Holds what's marched for a scene of `scene_size`.
    pub(super) fn new(device: &wgpu::Device, scene_size: (u32, u32)) -> Self {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let texture = |sample_type| wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen-space reflections"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            size_of::<ScreenReflectionUniform>() as u64
                        ),
                    },
                ),
                entry(1, texture(wgpu::TextureSampleType::Depth)),
                entry(
                    2,
                    texture(wgpu::TextureSampleType::Float { filterable: false }),
                ),
                entry(
                    3,
                    texture(wgpu::TextureSampleType::Float { filterable: true }),
                ),
                entry(
                    4,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(
                    5,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let shader = super::gpu::shader_module(
            device,
            "screen-space reflections",
            super::gpu::SCREEN_REFLECTIONS_SHADER,
        );
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("screen-space reflections"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("screen-space reflections"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                }),
            ),
            module: &shader,
            entry_point: Some("cs_screen_reflections"),
            compilation_options: Default::default(),
            cache: None,
        });
        let size = (scene_size.0.max(1), scene_size.1.max(1));
        Self {
            layout,
            pipeline,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("screen-space reflections"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screen-space reflections"),
                size: size_of::<ScreenReflectionUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screen-space reflections"),
                size: PIXEL_BYTES * u64::from(size.0) * u64::from(size.1),
                // Cleared on a frame with no last frame to read.
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            size,
            previous: None,
        }
    }

    /// Whether it holds what's marched for a scene of `scene_size`.
    pub(super) fn fits(&self, scene_size: (u32, u32)) -> bool {
        self.size == (scene_size.0.max(1), scene_size.1.max(1))
    }

    /// `Frame::ray_reflections` while it's drawn: the cutoff, one pixel
    /// for each, the width, and 1 to say they're found on the screen
    /// (faded by roughness more gently, and the sky's reflection shaded:
    /// lighting.wgsl's `traced_reflected`).
    pub(super) fn frame_values(&self) -> [f32; 4] {
        [ROUGHNESS_CUTOFF, 1.0, self.size.0 as f32, 1.0]
    }

    /// What the pass reads this frame, or `None` while there's no last
    /// frame to read: `drawn` is the scene as drawn this frame (jittered
    /// with TAA), `view` and `projection` the camera's, and `frame` which of
    /// TAA's frames this is (0 without TAA, so the steps hold still).
    pub(super) fn uniform(
        &self,
        drawn: Mat4,
        (view, projection): (Mat4, Mat4),
        frame: u64,
    ) -> Option<ScreenReflectionUniform> {
        let previous = self.previous?;
        let view_to_world = view.inverse();
        let camera = view_to_world.transform_point3(Vec3::ZERO);
        let forward = view_to_world
            .transform_vector3(Vec3::NEG_Z)
            .normalize_or_zero();
        // A depth `d` is `w / (d + z)` studs ahead, for the projection's
        // `z_axis.z` and `w_axis.z` (DirectX-style depth, 0 to 1).
        let depth_scale = projection.z_axis.z;
        let depth_offset = projection.w_axis.z;
        Some(ScreenReflectionUniform {
            clip_to_world: drawn.inverse().to_cols_array_2d(),
            world_to_clip: drawn.to_cols_array_2d(),
            previous: previous.to_cols_array_2d(),
            view_to_world: view_to_world.to_cols_array_2d(),
            camera: [camera.x, camera.y, camera.z, 0.0],
            forward: [forward.x, forward.y, forward.z, 0.0],
            size: [self.size.0, self.size.1, 1, (frame % 64) as u32],
            settings: [ROUGHNESS_CUTOFF, REACH, depth_scale, depth_offset],
        })
    }

    /// Records marching the reflections of the prepass in `targets` through
    /// last frame's picture `previous_scene` into `encoder`.
    pub(super) fn march(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        encoder: &mut wgpu::CommandEncoder,
        (targets, previous_scene): (&OcclusionTargets, &wgpu::TextureView),
        uniform: &ScreenReflectionUniform,
    ) {
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(uniform));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen-space reflections"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&targets.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(previous_scene),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.buffer.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("screen-space reflections"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(self.size.0.div_ceil(8), self.size.1.div_ceil(8), 1);
    }

    /// Bytes of its buffers.
    pub(super) fn bytes(&self) -> u64 {
        self.buffer.size() + self.uniform.size()
    }
}

/// Whether screen-space reflections are drawn with `graphics`: on, and ray
/// traced reflections (which take their place) aren't.
pub(super) fn marched(graphics: &EffectiveGraphics) -> bool {
    graphics.screen_space_reflections && !super::ray_reflections::traced(graphics)
}

#[cfg(test)]
mod tests {
    use glam::camera::rh;

    #[test]
    fn a_depth_is_turned_back_into_its_distance_from_the_camera() {
        // As the camera's (`lighting::camera_matrices`).
        let projection = rh::proj::directx::perspective(1.0, 1.5, 0.1, 5000.0);
        let (scale, offset) = (projection.z_axis.z, projection.w_axis.z);
        for distance in [0.5f32, 3.0, 40.0, 900.0] {
            let clip = projection * glam::Vec4::new(0.3, -0.2, -distance, 1.0);
            let depth = clip.z / clip.w;
            let back = offset / (depth + scale);
            assert!(
                (back - distance).abs() < distance * 1e-3,
                "{distance} came back as {back}"
            );
        }
    }
}
