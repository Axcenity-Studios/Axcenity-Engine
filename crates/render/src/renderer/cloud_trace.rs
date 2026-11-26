//! The traced clouds: the sky's clouds worked out in a pass of their own
//! (`cloud_trace.wgsl`), at half the frame's size and only where sky
//! shows, then laid over the sky by the sky pass (`fs_sky`).
//!
//! The picture is `clouded_sky`'s, which the sky pass used to work out for
//! each of its pixels: the same steps through the layer and toward the
//! sun, at every cloud quality that steps. Three things make it cheaper.
//!
//! - **Half size.** A cloud's finest detail is tens of studs across and
//!   hundreds of studs away, many pixels wide, so the light it adds and the
//!   sky it hides (`cloud_layer`) are worked out for every other pixel each
//!   way and read between. The sky behind them, the sun's disc and every
//!   silhouette in front stay at full size: the sky pass still draws only
//!   the pixels its depth test leaves, and multiplies its own sky in.
//! - **Only where sky shows.** The prepass's depth says which pixels no
//!   solid item covers. A texel none of the pixels that could read it
//!   needs is marked instead of traced, and the sky pass works a pixel out
//!   itself, as before, if it meets a mark after all.
//! - **Shapes from a texture.** `cloud_sum`, the five octaves of noise
//!   every step asks for (and each of its steps toward the sun), depends
//!   only on the place. It is kept in a texture (`FILL_SIZE` texels over
//!   `FILL_SPAN` studs, round the camera, carried with the clouds by the
//!   wind) and made again only when the camera has moved `RECENTRE` studs
//!   from its middle.
//!
//! Only while the prepass is drawn (its depth is what says where sky is)
//! and the cloud quality steps through the layer (Medium and up); otherwise
//! the sky pass draws them as before. Lit surfaces' cloud shadows don't
//! come through here. See `docs/rendering/clouds.md`.

use bytemuck::{Pod, Zeroable};

use super::clouds::CloudsRaw;
use super::gpu::{CLOUD_TRACE_SHADER, shader_module, target_bytes};
use super::types::{FrameUniform, HDR_FORMAT};

/// The texture of `cloud_sum`: its texels each way, and the studs it
/// covers (`CLOUD_FILL_SPAN` in clouds.wgsl, 12.8 studs a texel). A ray's
/// march ends at most `CLOUD_FAR + CLOUD_MARCH` (24,000 studs) away, and its
/// steps toward the sun reach 800 further.
const FILL_SIZE: u32 = 4096;
const FILL_SPAN: f64 = 52_428.8;
const FILL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
/// How far from the texture's middle the camera gets before it's made
/// again round the camera: the room `FILL_SPAN` leaves past 24,800 studs.
const RECENTRE: f64 = 1_200.0;

/// What the trace reads. Must match `Trace` in cloud_trace.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct TraceUniform {
    inv_view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    sun_direction: [f32; 4],
    sun_radiance: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    clouds: CloudsRaw,
}

/// The texture of `cloud_sum` and where its middle is.
struct Fill {
    view: wgpu::TextureView,
    /// Its middle, in studs from where the wind has carried the clouds;
    /// `None` until it's first drawn.
    centre: Option<[f64; 2]>,
    /// Whether this frame must draw it.
    stale: bool,
}

/// The targets, made for one set of frame targets.
struct Target {
    /// The frame's size.
    size: (u32, u32),
    /// The half-size trace.
    half: wgpu::TextureView,
    /// The frame-size clouds the sky pass reads.
    resolved: wgpu::TextureView,
    /// The fill texture and the prepass's depth, for both passes.
    trace_group: wgpu::BindGroup,
    /// The half-size trace, for the resolve.
    half_group: wgpu::BindGroup,
    /// The frame-size clouds, for the sky pass.
    sky_group: wgpu::BindGroup,
}

pub(super) struct CloudTrace {
    fill_pipeline: wgpu::RenderPipeline,
    trace_pipeline: wgpu::RenderPipeline,
    resolve_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    trace_layout: wgpu::BindGroupLayout,
    sky_layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    /// What the sky pass binds while nothing is traced: one texel it never
    /// reads.
    no_sky_group: wgpu::BindGroup,
    fill: Option<Fill>,
    target: Option<Target>,
    /// Whether this frame traces.
    traced: bool,
    /// Off, the sky pass draws the clouds itself whatever the settings
    /// (`Renderer::set_trace_clouds`, or `AXCENITY_NO_CLOUD_TRACE` set when
    /// the renderer is made: for comparing the two).
    pub(super) enabled: bool,
}

impl CloudTrace {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let shader = shader_module(device, "cloud trace", CLOUD_TRACE_SHADER);
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(ty),
            count: None,
        };
        let filtered = wgpu::TextureSampleType::Float { filterable: true };
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud trace uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let trace_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud trace"),
            entries: &[
                texture(0, filtered),
                sampler(1, wgpu::SamplerBindingType::Filtering),
                texture(2, wgpu::TextureSampleType::Depth),
                sampler(3, wgpu::SamplerBindingType::NonFiltering),
            ],
        });
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("traced clouds"),
            entries: &[texture(0, filtered)],
        });
        let pipeline = |label, layouts: &[Option<&wgpu::BindGroupLayout>], entry, format| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_screen"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let fill_pipeline = pipeline(
            "cloud shapes",
            &[Some(&uniform_layout)],
            "fs_fill",
            FILL_FORMAT,
        );
        let trace_pipeline = pipeline(
            "cloud trace",
            &[Some(&uniform_layout), Some(&trace_layout)],
            "fs_trace",
            HDR_FORMAT,
        );
        let resolve_pipeline = pipeline(
            "cloud resolve",
            &[
                Some(&uniform_layout),
                Some(&trace_layout),
                Some(&sky_layout),
            ],
            "fs_resolve",
            HDR_FORMAT,
        );
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud trace uniform"),
            size: size_of::<TraceUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud trace uniform"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let clamped = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let linear = clamped("cloud trace linear", wgpu::FilterMode::Linear);
        let nearest = clamped("cloud trace nearest", wgpu::FilterMode::Nearest);
        let no_clouds = target_view(device, "no traced clouds", (1, 1), HDR_FORMAT);
        let no_sky_group = sky_group(device, &sky_layout, &no_clouds);
        Self {
            fill_pipeline,
            trace_pipeline,
            resolve_pipeline,
            uniform,
            uniform_group,
            trace_layout,
            sky_layout,
            linear,
            nearest,
            no_sky_group,
            fill: None,
            target: None,
            traced: false,
            // A benchmark's way to time the sky pass's own clouds.
            enabled: std::env::var_os("AXCENITY_NO_CLOUD_TRACE").is_none(),
        }
    }

    /// The sky pass's bind group 2: the traced clouds.
    pub(super) fn sky_layout(&self) -> &wgpu::BindGroupLayout {
        &self.sky_layout
    }

    /// What the sky pass binds this frame.
    pub(super) fn sky_group(&self) -> &wgpu::BindGroup {
        match &self.target {
            Some(target) if self.traced => &target.sky_group,
            _ => &self.no_sky_group,
        }
    }

    /// Whether this frame traces the clouds.
    pub(super) fn traced(&self) -> bool {
        self.traced
    }

    /// Bytes of the textures kept.
    pub(super) fn bytes(&self) -> u64 {
        let fill = self.fill.as_ref().map_or(0, |_| {
            target_bytes((FILL_SIZE, FILL_SIZE, 1), FILL_FORMAT, 1)
        });
        let target = self.target.as_ref().map_or(0, |target| {
            let (width, height) = target.size;
            target_bytes((width, height, 1), HDR_FORMAT, 1)
                + target_bytes((width.div_ceil(2), height.div_ceil(2), 1), HDR_FORMAT, 1)
        });
        fill + target
    }

    /// Forgets the targets: the frame's targets, whose prepass depth they
    /// read, were made again.
    pub(super) fn forget_target(&mut self) {
        self.target = None;
    }

    /// Decides whether this frame's clouds are traced and gets ready to:
    /// `frame`'s clouds are told so (and where the fill texture's middle
    /// is), and the trace's own copy of what it reads is written. `drift` is
    /// how far the wind has carried the clouds, `size` the scene's in
    /// pixels and `prepass_depth` the prepass's depth, if it's drawn.
    /// Without clouds that step, or the prepass, nothing is traced and the
    /// textures are let go.
    pub(super) fn plan(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        frame: &mut FrameUniform,
        drift: [f64; 2],
        size: (u32, u32),
        prepass_depth: Option<&wgpu::TextureView>,
    ) {
        let clouds = &mut frame.clouds;
        let steps = clouds.color[3] != 0.0 && clouds.quality[0] > 0.0;
        let Some(depth) = prepass_depth.filter(|_| self.enabled && steps) else {
            self.traced = false;
            self.fill = None;
            self.target = None;
            return;
        };
        self.traced = true;

        // The fill texture follows the camera through the clouds' own
        // space, in whole texels so its samples stay where they were.
        let eye = [
            f64::from(frame.camera_position[0]) - drift[0],
            f64::from(frame.camera_position[2]) - drift[1],
        ];
        let fill = self.fill.get_or_insert_with(|| Fill {
            view: target_view(device, "cloud shapes", (FILL_SIZE, FILL_SIZE), FILL_FORMAT),
            centre: None,
            stale: true,
        });
        let centre = match fill.centre {
            Some(centre)
                if (eye[0] - centre[0]).abs() <= RECENTRE
                    && (eye[1] - centre[1]).abs() <= RECENTRE =>
            {
                centre
            }
            _ => {
                let texel = FILL_SPAN / f64::from(FILL_SIZE);
                let centre = eye.map(|studs| (studs / texel).round() * texel);
                fill.centre = Some(centre);
                fill.stale = true;
                // The target's group holds the fill texture, which stays.
                centre
            }
        };
        clouds.offset[2] = centre[0] as f32;
        clouds.offset[3] = centre[1] as f32;
        clouds.quality[3] = 1.0;

        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            let half_size = (size.0.div_ceil(2), size.1.div_ceil(2));
            let half = target_view(device, "half-size clouds", half_size, HDR_FORMAT);
            let resolved = target_view(device, "traced clouds", size, HDR_FORMAT);
            let trace_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud trace"),
                layout: &self.trace_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&fill.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.nearest),
                    },
                ],
            });
            self.target = Some(Target {
                size,
                half_group: sky_group(device, &self.sky_layout, &half),
                sky_group: sky_group(device, &self.sky_layout, &resolved),
                half,
                resolved,
                trace_group,
            });
        }
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&TraceUniform {
                inv_view_proj: frame.inv_view_proj,
                camera_position: frame.camera_position,
                sun_direction: frame.sun_direction,
                sun_radiance: frame.sun_radiance,
                sky_top: frame.sky_top,
                sky_horizon: frame.sky_horizon,
                clouds: frame.clouds,
            }),
        );
    }

    /// Draws the fill texture if the camera has left the last one, then
    /// traces the clouds. Nothing unless [`Self::plan`] decided to.
    pub(super) fn draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(fill), Some(target)) = (&mut self.fill, &self.target) else {
            return;
        };
        if !self.traced {
            return;
        }
        let begin = |encoder: &mut wgpu::CommandEncoder, label, view| {
            encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        depth_slice: None,
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime()
        };
        if std::mem::take(&mut fill.stale) {
            let mut pass = begin(encoder, "cloud shapes", &fill.view);
            pass.set_pipeline(&self.fill_pipeline);
            pass.set_bind_group(0, &self.uniform_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = begin(encoder, "cloud trace", &target.half);
        pass.set_pipeline(&self.trace_pipeline);
        pass.set_bind_group(0, &self.uniform_group, &[]);
        pass.set_bind_group(1, &target.trace_group, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        let mut pass = begin(encoder, "cloud resolve", &target.resolved);
        pass.set_pipeline(&self.resolve_pipeline);
        pass.set_bind_group(0, &self.uniform_group, &[]);
        pass.set_bind_group(1, &target.trace_group, &[]);
        pass.set_bind_group(2, &target.half_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// A texture drawn into and read, and its view.
fn target_view(
    device: &wgpu::Device,
    label: &str,
    (width, height): (u32, u32),
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn sky_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    clouds: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("traced clouds"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(clouds),
        }],
    })
}
