//! A scatter item's masks: where on the terrain the Scatter brush places
//! more or less of it. Each mask is a number from 0 (none) to 1 (as many
//! as the brush's density says), and an item's chance at a spot is its
//! masks multiplied together ([`ScatterMask::density`]):
//!
//! - **noise**: patches, so an item gathers in clumps instead of covering
//!   the ground evenly;
//! - **slope** and **height**: only between two slopes or two heights;
//! - **moisture**: worked out from the terrain's own shape. A hollow (the
//!   ground round a spot is higher than the spot: a wash, a ditch, the
//!   foot of a slope) counts as wet, a rise (a ridge, a knoll) as dry, and
//!   flat or evenly sloping ground as in between. An item's bias then
//!   prefers the wet or the dry.
//!
//! A new item's masks are all open ([`ScatterMask::is_open`]): the brush
//! then places it exactly as it did before masks existed, and draws no
//! random numbers for them.

use axcenity_core::TerrainData;
use glam::Vec2;

use crate::Placement;
use crate::noise::{Noise, mix, smoothstep};

/// At or past this height (studs, world Y; and at or below its negative),
/// the height mask has no top (or bottom).
pub const OPEN_HEIGHT: f32 = 10_000.0;
/// Degrees over which the slope mask fades in and out at its two ends.
pub const SLOPE_FADE: f32 = 5.0;
/// Studs over which the height mask fades in and out at its two ends.
pub const HEIGHT_FADE: f32 = 4.0;
/// How much higher (or lower) than a spot the ground round it has to be on
/// average, as a share of the moisture reach, for the spot to count as
/// fully wet (or dry): 0.8 studs with a reach of 16, which the middle of a
/// wash 3 studs deep and 40 wide nearly is.
pub const WET_RELIEF: f32 = 0.05;
/// The moisture reach's range, in studs.
pub const MOISTURE_REACH: (f32, f32) = (1.0, 256.0);
/// The noise scale's range, in studs.
pub const NOISE_SCALE: (f32, f32) = (1.0, 2_000.0);

/// Where a scatter item is placed more or less densely.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterMask {
    /// 0 (off) to 1: how much the noise thins the item out between its
    /// patches. At 1 there's none between them.
    pub noise: f32,
    /// Studs across a patch.
    pub noise_scale: f32,
    /// 0 to 1: how much of the ground the patches leave bare. 0.5 is
    /// about half.
    pub noise_threshold: f32,
    /// 0 to 1: how sharp the patches' edges are. 0 fades across a whole
    /// patch, 1 is a hard edge.
    pub noise_contrast: f32,
    /// Which patches: another seed moves them all.
    pub noise_seed: u32,
    /// The gentlest and steepest ground it's placed on, in degrees (0
    /// flat, 90 a wall).
    pub min_slope: f32,
    pub max_slope: f32,
    /// The lowest and highest ground it's placed on, in studs (world Y).
    pub min_height: f32,
    pub max_height: f32,
    /// -1 to 1: above 0 it prefers wet hollows, below 0 dry rises; 0
    /// doesn't look at moisture.
    pub moisture: f32,
    /// Studs round a spot the ground is compared with, to tell a hollow
    /// from a rise: about half the width of the washes to pick out.
    pub moisture_reach: f32,
}

impl Default for ScatterMask {
    fn default() -> Self {
        Self {
            noise: 0.0,
            noise_scale: 40.0,
            noise_threshold: 0.5,
            noise_contrast: 0.5,
            noise_seed: 1,
            min_slope: 0.0,
            max_slope: 90.0,
            min_height: -OPEN_HEIGHT,
            max_height: OPEN_HEIGHT,
            moisture: 0.0,
            moisture_reach: 16.0,
        }
    }
}

/// `value` if it's a number, else `fallback`.
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

impl ScatterMask {
    /// Whether the noise mask does anything.
    pub fn has_noise(&self) -> bool {
        finite(self.noise, 0.0) > 0.0
    }

    /// The slope mask's two ends in order, or `None` when it's open (0 to
    /// 90 degrees).
    pub fn slope_range(&self) -> Option<(f32, f32)> {
        let (a, b) = (
            finite(self.min_slope, 0.0).clamp(0.0, 90.0),
            finite(self.max_slope, 90.0).clamp(0.0, 90.0),
        );
        let (low, high) = (a.min(b), a.max(b));
        (low > 0.0 || high < 90.0).then_some((low, high))
    }

    /// The height mask's two ends in order, or `None` when it's open.
    pub fn height_range(&self) -> Option<(f32, f32)> {
        let (a, b) = (
            finite(self.min_height, -OPEN_HEIGHT).clamp(-OPEN_HEIGHT, OPEN_HEIGHT),
            finite(self.max_height, OPEN_HEIGHT).clamp(-OPEN_HEIGHT, OPEN_HEIGHT),
        );
        let (low, high) = (a.min(b), a.max(b));
        (low > -OPEN_HEIGHT || high < OPEN_HEIGHT).then_some((low, high))
    }

    /// Whether the moisture mask does anything.
    pub fn has_moisture(&self) -> bool {
        finite(self.moisture, 0.0) != 0.0
    }

    /// True when no mask does anything: the item is placed wherever the
    /// brush goes, as before masks existed.
    pub fn is_open(&self) -> bool {
        !self.has_noise()
            && self.slope_range().is_none()
            && self.height_range().is_none()
            && !self.has_moisture()
    }

    /// 0 to 1: the noise mask at world (`x`, `z`). 1 inside a patch.
    pub fn noise_at(&self, at: Vec2) -> f32 {
        let amount = finite(self.noise, 0.0).clamp(0.0, 1.0);
        if amount <= 0.0 {
            return 1.0;
        }
        let scale = finite(self.noise_scale, 40.0).clamp(NOISE_SCALE.0, NOISE_SCALE.1);
        let n = Noise::new(u64::from(self.noise_seed) ^ 0x5CA7_7E12_0000_0000).fbm(
            at.x / scale,
            at.y / scale,
            3,
            0.5,
            2.0,
        );
        // Fractal noise stays mostly within ±0.6, so that's what the
        // threshold's 0 to 1 is spread over.
        let level = (finite(self.noise_threshold, 0.5).clamp(0.0, 1.0) - 0.5) * 1.2;
        let contrast = finite(self.noise_contrast, 0.5).clamp(0.0, 1.0);
        let half = 0.5 + (0.01 - 0.5) * contrast;
        let inside = smoothstep(level - half, level + half, n);
        1.0 - amount * (1.0 - inside)
    }

    /// 0 to 1: the slope mask for ground `slope` degrees steep. An end at
    /// 0 or 90 is open, so a mask from 0 covers flat ground fully.
    pub fn slope_at(&self, slope: f32) -> f32 {
        let Some((low, high)) = self.slope_range() else {
            return 1.0;
        };
        window(slope, (low, high), SLOPE_FADE, (0.0, 90.0))
    }

    /// 0 to 1: the height mask for ground at `y` studs (world Y).
    pub fn height_at(&self, y: f32) -> f32 {
        let Some((low, high)) = self.height_range() else {
            return 1.0;
        };
        window(y, (low, high), HEIGHT_FADE, (-OPEN_HEIGHT, OPEN_HEIGHT))
    }

    /// 0 to 1: the moisture mask for ground whose [`wetness`] is `wet`.
    /// A bias of 1 keeps as many as the ground is wet (all in a hollow,
    /// half on the flat, none on a rise); -1 the other way round.
    pub fn moisture_at(&self, wet: f32) -> f32 {
        let bias = finite(self.moisture, 0.0).clamp(-1.0, 1.0);
        let wet = wet.clamp(0.0, 1.0);
        if bias >= 0.0 {
            1.0 - bias * (1.0 - wet)
        } else {
            1.0 + bias * wet
        }
    }

    /// 0 to 1: the share of the brush's density the item gets at sample
    /// coordinates (`gx`, `gz`) of `data`: its masks multiplied together.
    pub fn density(&self, data: &TerrainData, placement: &Placement, gx: f32, gz: f32) -> f32 {
        if self.is_open() {
            return 1.0;
        }
        let at = placement.to_world_xz(data, Vec2::new(gx, gz));
        let mut density = self.noise_at(at);
        if density <= 0.0 {
            return 0.0;
        }
        if self.height_range().is_some() {
            density *= self.height_at(placement.world_y(data.height_at(gx, gz)));
        }
        if self.slope_range().is_some() {
            let (size_x, size_z) = data.size();
            let (nx, nz) = (
                (gx.round().max(0.0) as usize).min(size_x - 1),
                (gz.round().max(0.0) as usize).min(size_z - 1),
            );
            density *= self.slope_at(placement.slope_degrees(data, nx, nz));
        }
        if density > 0.0 && self.has_moisture() {
            let reach = finite(self.moisture_reach, 16.0).clamp(MOISTURE_REACH.0, MOISTURE_REACH.1);
            density *= self.moisture_at(wetness(data, placement, gx, gz, reach));
        }
        density.clamp(0.0, 1.0)
    }
}

/// 0 to 1: how far `value` is inside `range`, fading over `fade` centered
/// on each end. An end at `open`'s is open: nothing fades there.
fn window(value: f32, (low, high): (f32, f32), fade: f32, open: (f32, f32)) -> f32 {
    let half = fade / 2.0;
    let above = if low <= open.0 {
        1.0
    } else {
        smoothstep(low - half, low + half, value)
    };
    let below = if high >= open.1 {
        1.0
    } else {
        1.0 - smoothstep(high - half, high + half, value)
    };
    above * below
}

/// How wet the ground is at sample coordinates (`gx`, `gz`), from its
/// shape alone: 0 on a rise, 0.5 on flat or evenly sloping ground, 1 in a
/// hollow. It's the mean height of two rings round the spot (`reach` studs
/// and half that away, eight points each) less the spot's own height, as a
/// share of `reach`: fully wet (or dry) from [`WET_RELIEF`]. Past the
/// terrain's edge the rings read the edge's height.
pub fn wetness(data: &TerrainData, placement: &Placement, gx: f32, gz: f32, reach: f32) -> f32 {
    let cell = placement.cell_size(data);
    let reach = reach.max(1e-3);
    let here = data.height_at(gx, gz);
    let mut sum = 0.0;
    for ring in [1.0_f32, 0.5] {
        for step in 0..8 {
            // The half-reach ring is turned half a step, so the sixteen
            // points look sixteen ways.
            let angle =
                (step as f32 + if ring < 1.0 { 0.5 } else { 0.0 }) * std::f32::consts::TAU / 8.0;
            let offset = Vec2::new(angle.cos(), angle.sin()) * reach * ring;
            sum += data.height_at(
                gx + offset.x / cell.x.max(1e-6),
                gz + offset.y / cell.y.max(1e-6),
            );
        }
    }
    // Heights are 0 to 1 of the terrain's height range.
    let relief = (sum / 16.0 - here) * placement.size.y / reach;
    smoothstep(-WET_RELIEF, WET_RELIEF, relief)
}

/// A number from 0 up to (not including) 1 for a spot, the same for the
/// same stroke seed and spot: what a masked item's chance is rolled
/// against, apart from the stroke's own random numbers.
pub(super) fn roll(seed: u64, gx: f32, gz: f32) -> f32 {
    let hash = mix(seed
        ^ 0x4D41_534B_0000_0000
        ^ u64::from(gx.to_bits())
        ^ u64::from(gz.to_bits()).rotate_left(32));
    (hash >> 40) as f32 / (1u64 << 24) as f32
}

#[cfg(test)]
#[path = "scatter_mask_tests.rs"]
mod tests;
