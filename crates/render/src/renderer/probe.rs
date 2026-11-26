//! A measuring switch for scattered items (a terrain's painted plants,
//! stones and litter) in the scene pass: `AXCENITY_PLANT_PROBE`.
//!
//! The scene pass spends four to six times the prepass's time on the same
//! triangles, whatever the window's size. To tell what that time is spent
//! on, the probe draws those items' pixels with part of the work left out
//! (`fs_probe_*` in shader.wgsl), and `--bench`'s "Scene: scattered items"
//! then reads what is left:
//!
//! - `flat`: nothing is carried to a corner but its position, and a pixel
//!   costs nothing. What remains is the corners' own work and the
//!   rasterizer's.
//! - `carried`: everything the vertex shader carries is carried and read,
//!   and nothing is done with it.
//! - `surface`: the material's textures are read, and the surface is not
//!   lit.
//!
//! The picture is wrong while it is on, so it is read from the environment
//! when the pipelines are made and nothing else sets it
//! (`docs/rendering/performance.md`).

use super::gpu::{DepthUse, SCENE_SHADER, create_pipeline_for, shader_module};
use super::gpu_timer::DrawnPart;
use super::{Batch, HDR_FORMAT};

/// The probe's pipelines, by `double_sided`.
pub(super) struct PlantProbe {
    /// For items drawn with [`DepthUse::Plain`].
    plain: [wgpu::RenderPipeline; 2],
    /// For items whose depth is already there ([`DepthUse::Filled`]).
    filled: [wgpu::RenderPipeline; 2],
}

/// The fragment shader `asked` (the variable's value) names.
fn fragment_of(asked: &str) -> Option<&'static str> {
    match asked {
        "flat" => Some("fs_probe_flat"),
        "carried" => Some("fs_probe_carried"),
        "surface" => Some("fs_probe_surface"),
        _ => None,
    }
}

impl PlantProbe {
    /// The probe `AXCENITY_PLANT_PROBE` asks for, if any, for a scene
    /// drawn with `samples` per pixel.
    pub(super) fn from_environment(
        device: &wgpu::Device,
        layouts: [&wgpu::BindGroupLayout; 3],
        samples: u32,
    ) -> Option<Self> {
        let asked = std::env::var("AXCENITY_PLANT_PROBE").ok()?;
        let Some(fragment) = fragment_of(&asked) else {
            log::warn!("AXCENITY_PLANT_PROBE: `{asked}` is not flat, carried or surface");
            return None;
        };
        log::warn!("AXCENITY_PLANT_PROBE={asked}: scattered items are not drawn as they should be");
        let shader = shader_module(device, "plant probe", SCENE_SHADER);
        let pipeline = |double_sided, depth| {
            create_pipeline_for(
                device,
                &shader,
                ("vs_main", fragment),
                &layouts,
                (HDR_FORMAT, samples),
                (false, double_sided),
                depth,
            )
        };
        Some(Self {
            plain: [
                pipeline(false, DepthUse::Plain),
                pipeline(true, DepthUse::Plain),
            ],
            filled: [
                pipeline(false, DepthUse::Filled),
                pipeline(true, DepthUse::Filled),
            ],
        })
    }

    /// The pipeline `batch` is drawn with in the scene pass while the
    /// probe is on, or `None` for a batch it leaves alone.
    pub(super) fn pipeline(&self, batch: &Batch) -> Option<&wgpu::RenderPipeline> {
        let probed = batch.part == DrawnPart::Scattered
            && !batch.transparent
            && !batch.water
            && batch.shader.is_none();
        let side = usize::from(batch.double_sided);
        probed.then(|| {
            if batch.prefilled {
                &self.filled[side]
            } else {
                &self.plain[side]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_probe_names_a_fragment_shader_of_the_scene_shader() {
        for asked in ["flat", "carried", "surface"] {
            let fragment = fragment_of(asked).expect("a probe");
            assert!(
                SCENE_SHADER.contains(&format!("fn {fragment}(")),
                "{fragment} is not in the scene shader"
            );
        }
        assert_eq!(fragment_of("on"), None);
    }
}
