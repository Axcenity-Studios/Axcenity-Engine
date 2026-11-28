//! A measuring switch for occlusion culling: `AXCENITY_OCCLUSION_PROBE`.
//!
//! Before building a Hi-Z pass (a depth pyramid that culls hidden items
//! on the GPU), this says what one could save in a real scene. On the frame
//! it names, the prepass's depth is read back to the CPU once, and from
//! then on each draw the camera would see is left out of the camera's
//! passes (the prepass, the scene, motion) when its bounding sphere lies
//! wholly behind that depth. Shadow casters are kept: what the camera can't
//! see still shades what the sun's cascades cover. For a camera that holds
//! still (a `--bench` at the spawn, a still) that leaves out exactly what a
//! perfect occlusion test would, with the same picture, so `--bench`'s
//! times with and without it are the most culling hidden items can save.
//! A camera that moves sees a stale depth and loses items: it is only for
//! measuring, read from the environment, and nothing else sets it.
//!
//! `AXCENITY_OCCLUSION_PROBE=30` reads the depth on frame 30;
//! `=30/64` on frame 30 and every 64 frames after it (the still tool's
//! shots, each its own view). Each read prints how many of the camera's
//! draws its depth hides (`docs/rendering/performance.md`).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use glam::{Mat4, Vec3, Vec4};

use super::culling::Sphere;

/// The frames the probe reads the depth on: the first and, when given,
/// how many frames apart the next ones are.
fn schedule() -> Option<(u64, Option<u64>)> {
    static ASKED: OnceLock<Option<(u64, Option<u64>)>> = OnceLock::new();
    *ASKED.get_or_init(|| {
        let text = std::env::var("AXCENITY_OCCLUSION_PROBE").ok()?;
        parse_schedule(&text)
    })
}

/// `"30"` or `"30/64"`.
fn parse_schedule(text: &str) -> Option<(u64, Option<u64>)> {
    let mut parts = text.trim().splitn(2, '/');
    let first = parts.next()?.trim().parse().ok()?;
    let every = match parts.next() {
        Some(every) => Some(every.trim().parse().ok().filter(|n| *n > 0)?),
        None => None,
    };
    Some((first, every))
}

/// Whether the depth is read on `frame`.
pub(super) fn reads_on(frame: u64) -> bool {
    match schedule() {
        Some((first, None)) => frame == first,
        Some((first, Some(every))) => frame >= first && (frame - first).is_multiple_of(every),
        None => false,
    }
}

/// The depth last read, while the probe is on.
fn kept() -> &'static Mutex<Option<Arc<DepthImage>>> {
    static KEPT: OnceLock<Mutex<Option<Arc<DepthImage>>>> = OnceLock::new();
    KEPT.get_or_init(|| Mutex::new(None))
}

/// Frames planned since the depth was last read.
static PLANNED: AtomicU32 = AtomicU32::new(0);

/// The depth to test the camera's draws against, if the probe has read one.
pub(super) fn image() -> Option<Arc<DepthImage>> {
    schedule()?;
    kept().lock().ok()?.clone()
}

/// Counts `hidden` draws left out this frame: printed on the eighth frame
/// planned with a depth just read.
pub(super) fn count(hidden: u32) {
    if PLANNED.fetch_add(1, Ordering::Relaxed) == 7 {
        eprintln!("occlusion probe: the depth hides {hidden} of the camera's draws");
    }
}

/// A depth image (0 near, 1 far, as wgpu's clip space) and its farthest
/// depth over each 2 × 2, 4 × 4, … block: a CPU depth pyramid.
pub(super) struct DepthImage {
    view_proj: Mat4,
    /// Level 0 is the image itself; each next level halves it, rounding up.
    levels: Vec<Level>,
}

struct Level {
    width: u32,
    height: u32,
    farthest: Vec<f32>,
}

impl Level {
    fn at(&self, x: u32, y: u32) -> f32 {
        self.farthest[(y.min(self.height - 1) * self.width + x.min(self.width - 1)) as usize]
    }

    fn halved(&self) -> Level {
        let (width, height) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let mut farthest = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let (x0, y0) = (x * 2, y * 2);
                farthest.push(
                    self.at(x0, y0)
                        .max(self.at(x0 + 1, y0))
                        .max(self.at(x0, y0 + 1))
                        .max(self.at(x0 + 1, y0 + 1)),
                );
            }
        }
        Level {
            width,
            height,
            farthest,
        }
    }
}

/// How many texels across a test reads at most, at the level it picks.
const TEST_TEXELS: u32 = 8;

impl DepthImage {
    /// `depth` row by row, `width` × `height`, seen through `view_proj`.
    pub(super) fn new(view_proj: Mat4, width: u32, height: u32, depth: Vec<f32>) -> Self {
        assert_eq!(depth.len(), (width * height) as usize);
        let mut levels = vec![Level {
            width: width.max(1),
            height: height.max(1),
            farthest: depth,
        }];
        while let Some(last) = levels.last()
            && (last.width > 1 || last.height > 1)
        {
            let next = last.halved();
            levels.push(next);
        }
        Self { view_proj, levels }
    }

    /// Whether everything within `sphere` is behind the depth: the nearest
    /// corner of the box around it is farther than the farthest depth over
    /// the pixels the box covers (one more pixel each way, for TAA's
    /// jitter). A box reaching the near plane or off the image is not.
    pub(super) fn hides(&self, (center, radius): Sphere) -> bool {
        let base = &self.levels[0];
        let (width, height) = (base.width as f32, base.height as f32);
        let mut nearest = f32::MAX;
        let (mut low, mut high) = (glam::Vec2::splat(f32::MAX), glam::Vec2::splat(f32::MIN));
        for corner in 0..8 {
            let sign = |bit: u32| if corner & bit == 0 { -1.0 } else { 1.0 };
            let at = center + Vec3::new(sign(1), sign(2), sign(4)) * radius;
            let clip = self.view_proj * Vec4::new(at.x, at.y, at.z, 1.0);
            if clip.w <= 1e-4 {
                return false;
            }
            let ndc = clip.truncate() / clip.w;
            if ndc.z <= 0.0 {
                return false;
            }
            nearest = nearest.min(ndc.z);
            // Pixels, Y down.
            let pixel = glam::Vec2::new((ndc.x * 0.5 + 0.5) * width, (0.5 - ndc.y * 0.5) * height);
            low = low.min(pixel);
            high = high.max(pixel);
        }
        if !nearest.is_finite() || low.x.is_nan() || high.x.is_nan() {
            return false;
        }
        // Off the image on a side: what's past the edge isn't known.
        let low = (low - 1.0).max(glam::Vec2::ZERO);
        let high = (high + 1.0).min(glam::Vec2::new(width - 1.0, height - 1.0));
        if high.x < low.x || high.y < low.y {
            return false;
        }
        let (x0, y0, x1, y1) = (low.x as u32, low.y as u32, high.x as u32, high.y as u32);
        // The finest level where the box spans at most `TEST_TEXELS` texels.
        let span = (x1 - x0 + 1).max(y1 - y0 + 1);
        let mut level = 0;
        while level + 1 < self.levels.len() && (span >> level) > TEST_TEXELS {
            level += 1;
        }
        let image = &self.levels[level];
        for y in (y0 >> level)..=(y1 >> level) {
            for x in (x0 >> level)..=(x1 >> level) {
                if image.at(x, y) >= nearest {
                    return false;
                }
            }
        }
        true
    }
}

/// Reads `texture` (the prepass's depth, `Depth32Float`) back, and keeps it
/// for the next frames' draws. `view_proj` is the camera it was drawn from.
pub(super) fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    view_proj: Mat4,
) {
    let size = texture.size();
    let (width, height) = (size.width, size.height);
    let row_bytes = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("occlusion probe"),
        size: u64::from(row_bytes) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("occlusion probe"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::DepthOnly,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    if device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
        return;
    }
    let mut depth = Vec::with_capacity((width * height) as usize);
    {
        let Ok(bytes) = buffer.slice(..).get_mapped_range() else {
            return;
        };
        for row in bytes.chunks_exact(row_bytes as usize) {
            depth.extend(
                row[..(width * 4) as usize]
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            );
        }
    }
    PLANNED.store(0, Ordering::Relaxed);
    eprintln!("occlusion probe: read the prepass's depth ({width} x {height})");
    if let Ok(mut kept) = kept().lock() {
        *kept = Some(Arc::new(DepthImage::new(view_proj, width, height, depth)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Camera;
    use crate::renderer::lighting::camera_view_projection;

    fn camera() -> Camera {
        Camera {
            position: Vec3::new(0.0, 0.0, 10.0),
            look_at: Vec3::ZERO,
            fov_y_degrees: 60.0,
        }
    }

    /// A wall's depth across the whole image, `at` studs from the camera.
    fn wall(at: f32) -> DepthImage {
        let view_proj = camera_view_projection(&camera(), 1.0);
        let clip = view_proj * Vec4::new(0.0, 0.0, 10.0 - at, 1.0);
        DepthImage::new(view_proj, 64, 64, vec![clip.z / clip.w; 64 * 64])
    }

    #[test]
    fn the_schedule_reads_once_or_every_so_many_frames() {
        assert_eq!(parse_schedule("30"), Some((30, None)));
        assert_eq!(parse_schedule(" 30 / 64 "), Some((30, Some(64))));
        assert_eq!(parse_schedule("30/0"), None);
        assert_eq!(parse_schedule("soon"), None);
    }

    #[test]
    fn what_is_behind_a_wall_is_hidden_and_what_is_before_it_is_not() {
        let image = wall(5.0);
        // Behind the wall (z = 5), wholly.
        assert!(image.hides((Vec3::new(0.0, 0.0, -2.0), 1.0)));
        // In front of it.
        assert!(!image.hides((Vec3::new(0.0, 0.0, 8.0), 1.0)));
        // Reaching through it.
        assert!(!image.hides((Vec3::new(0.0, 0.0, 4.5), 1.0)));
        // Behind the camera, or reaching it.
        assert!(!image.hides((Vec3::new(0.0, 0.0, 20.0), 1.0)));
        assert!(!image.hides((Vec3::new(0.0, 0.0, 10.0), 1.0)));
    }

    #[test]
    fn a_hole_in_the_wall_shows_what_is_behind_it() {
        let view_proj = camera_view_projection(&camera(), 1.0);
        let clip = view_proj * Vec4::new(0.0, 0.0, 5.0, 1.0);
        let mut depth = vec![clip.z / clip.w; 64 * 64];
        // One far pixel (the sky) in the middle.
        depth[32 * 64 + 32] = 1.0;
        let image = DepthImage::new(view_proj, 64, 64, depth);
        assert!(!image.hides((Vec3::new(0.0, 0.0, -2.0), 1.0)));
        // Off to the side, away from the hole, it is still hidden.
        assert!(image.hides((Vec3::new(4.0, 4.0, -2.0), 0.5)));
    }
}
