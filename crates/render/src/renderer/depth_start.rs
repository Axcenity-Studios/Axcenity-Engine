//! Where the scene pass's depth starts: [`DepthStart`].
//!
//! The scene pass used to clear its depth to the far plane, so a surface
//! was lit whenever nothing nearer had been drawn *yet*: what order allowed
//! was lit and then drawn over. In Desert Homestead that was over a third
//! of the scene pass (`docs/rendering/performance.md`, "The scene pass's
//! depth from the prepass").
//!
//! The depth and normal prepass has already drawn the same solid items
//! from the same corners (their positions are `@invariant`, so both passes
//! give a corner exactly the same depth). So the scene's depth starts from
//! the prepass's, one representable step farther (`depth_start.wgsl`).
//! Every pipeline, test and order of the scene pass stays as it is:
//!
//! - what the prepass kept is still nearer than what's there, so it is
//!   drawn and writes its own depth, and a second surface at exactly that
//!   depth then fails, as before: the same one of them shows;
//! - what's farther by a step or more fails the test before it is lit.
//!
//! The picture is the same, pixel for pixel. It needs the prepass to have
//! drawn the scene's solid items with as many samples a pixel: one, so not
//! under MSAA, and not while nothing reads the prepass (no ambient
//! occlusion, contact shadows, TAA or anything else that keeps it).
//! Elsewhere the scene pass clears its depth as before.

use super::DEPTH_FORMAT;

pub(super) struct DepthStart {
    /// Off draws the scene pass from a cleared depth everywhere
    /// (`Renderer::set_depth_from_prepass`, or `AXCENITY_NO_DEPTH_START`
    /// set when the renderer is made), for comparing the two.
    pub(super) enabled: bool,
    /// Whether cut-outs are lit straight over the started depth with no
    /// fill first (`PrefillPipelines::tested`). Off (`AXCENITY_CUT_OUT_FILL`
    /// set when the renderer is made) draws their depth first as before,
    /// for comparing the two.
    pub(super) tested_cut_outs: bool,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// The prepass's depth last read, and its binding.
    bound: Option<(wgpu::TextureView, wgpu::BindGroup)>,
}

impl DepthStart {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth start"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depth start"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("../depth_start.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth start"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            // Depth alone: the shader gives each pixel's.
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            enabled: std::env::var_os("AXCENITY_NO_DEPTH_START").is_none(),
            tested_cut_outs: std::env::var_os("AXCENITY_CUT_OUT_FILL").is_none(),
            pipeline,
            layout,
            bound: None,
        }
    }

    /// Draws `scene_depth`'s start from `prepass_depth`, which must be
    /// the depth of the same solid items at the same size with one sample
    /// a pixel. False, with nothing drawn, while it's off: the scene pass
    /// then clears its depth.
    pub(super) fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        prepass_depth: &wgpu::TextureView,
        scene_depth: &wgpu::TextureView,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        if self
            .bound
            .as_ref()
            .is_none_or(|(view, _)| view != prepass_depth)
        {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("depth start"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(prepass_depth),
                }],
            });
            self.bound = Some((prepass_depth.clone(), group));
        }
        let Some((_, group)) = &self.bound else {
            return false;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth start"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: scene_depth,
                depth_ops: Some(wgpu::Operations {
                    // Every pixel is written.
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}
