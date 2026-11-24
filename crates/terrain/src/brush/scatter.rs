//! The Scatter brush: one stroke places a weighted random mix of a
//! palette's items on the terrain (as painted foliage), each with its own
//! size range, turn, collide, lean and sink, or erases them.
//!
//! Placement is deterministic: each stamp's random numbers come from the
//! stroke's seed and the stamp's middle, so the same seed and path give the
//! same items. A minimum spacing keeps a new item that many studs from any
//! painted foliage already there (and from the others the stamp adds), so
//! going over the same ground again fills gaps instead of piling items up.
//!
//! An item's masks (`scatter_mask.rs`: noise patches, slope, height and
//! moisture) thin it out where they say: a spot picked for a masked item
//! is kept with the chance its masks give, rolled against a number of the
//! spot's own, so the stroke's random numbers (and so every unmasked
//! item) are the same with or without masks.

use std::collections::HashMap;

use axcenity_core::{
    FoliageInstance, FoliageSource, MAX_FOLIAGE_INSTANCES, SampleRect, TerrainData,
};
use glam::Vec2;

use super::scatter_mask::{ScatterMask, roll as mask_roll};
use super::{Brush, MAX_FOLIAGE_PER_STAMP};
use crate::Placement;
use crate::noise::{Rng, mix};

/// The largest size an item can be scaled to.
pub const MAX_ITEM_SCALE: f32 = 100.0;
/// The smallest.
pub const MIN_ITEM_SCALE: f32 = 0.01;

/// One entry of a scatter palette: what to place and how.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterItem {
    pub source: FoliageSource,
    /// Off, it isn't placed (or erased by Shift).
    pub enabled: bool,
    /// Its share of the mix, against the other enabled items' weights.
    pub weight: f32,
    /// The smallest and largest scale it's placed at (1 is its own size).
    pub min_scale: f32,
    pub max_scale: f32,
    /// Whether parts rest on it (solid) or fall through it (visual only).
    pub collide: bool,
    /// How far it's turned at random about the up axis, in degrees: 0
    /// faces every copy the same way, 360 any way.
    pub rotation: f32,
    /// 0 (upright) to 1 (square to the ground): [`FoliageInstance::lean`].
    pub align_to_slope: f32,
    /// Studs it's lowered into the ground, whatever its scale:
    /// [`FoliageInstance::sink`].
    pub sink: f32,
    /// Where it's placed more or less densely: noise patches, slope,
    /// height and moisture. Open (the default), it's placed everywhere.
    pub mask: ScatterMask,
}

impl ScatterItem {
    /// `source` with the usual settings: on, weight 1, a little size
    /// variation, any turn, visual only, upright and on the surface.
    pub fn new(source: FoliageSource) -> Self {
        Self {
            source,
            enabled: true,
            weight: 1.0,
            min_scale: 0.85,
            max_scale: 1.15,
            collide: false,
            rotation: 360.0,
            align_to_slope: 0.0,
            sink: 0.0,
            mask: ScatterMask::default(),
        }
    }

    /// Whether a stroke places it: on, with a weight above 0.
    pub fn places(&self) -> bool {
        self.enabled && self.weight.is_finite() && self.weight > 0.0
    }

    /// Its smallest and largest scale, in order and in range.
    pub fn scale_range(&self) -> (f32, f32) {
        let clean = |value: f32| {
            if value.is_finite() {
                value.clamp(MIN_ITEM_SCALE, MAX_ITEM_SCALE)
            } else {
                1.0
            }
        };
        let (a, b) = (clean(self.min_scale), clean(self.max_scale));
        (a.min(b), a.max(b))
    }
}

/// Each item's share of a palette's mix, 0 to 1: its weight over the
/// enabled items' total (0 for one that's off).
pub fn shares(items: &[ScatterItem]) -> Vec<f32> {
    let total: f32 = items
        .iter()
        .filter(|item| item.places())
        .map(|item| item.weight)
        .sum();
    items
        .iter()
        .map(|item| {
            if item.places() && total > 0.0 {
                item.weight / total
            } else {
                0.0
            }
        })
        .collect()
}

/// What Shift (erasing) takes away under the brush.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScatterErase {
    /// The palette's enabled items only.
    #[default]
    Palette,
    /// Any painted foliage.
    Everything,
}

/// One Scatter stamp's settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Scatter {
    pub items: Vec<ScatterItem>,
    /// Items per square stud at full strength.
    pub density: f32,
    /// Studs kept between a new item and any painted foliage (0: none).
    pub spacing: f32,
    /// Erase instead of placing.
    pub erase: Option<ScatterErase>,
    /// Where the stroke's random numbers start.
    pub seed: u64,
}

impl Scatter {
    /// Whether `source` is one of the enabled items.
    pub fn has(&self, source: &FoliageSource) -> bool {
        self.items
            .iter()
            .any(|item| item.enabled && item.source == *source)
    }
}

/// Positions (world X, Z) bucketed by `cell` studs, to find neighbours
/// closer than the spacing quickly.
struct Neighbours {
    cell: f32,
    buckets: HashMap<(i32, i32), Vec<Vec2>>,
}

impl Neighbours {
    fn new(cell: f32) -> Self {
        Self {
            cell: cell.max(0.01),
            buckets: HashMap::new(),
        }
    }

    fn key(&self, at: Vec2) -> (i32, i32) {
        (
            (at.x / self.cell).floor() as i32,
            (at.y / self.cell).floor() as i32,
        )
    }

    fn add(&mut self, at: Vec2) {
        let key = self.key(at);
        self.buckets.entry(key).or_default().push(at);
    }

    /// True if something is within `distance` (at most the bucket size).
    fn near(&self, at: Vec2, distance: f32) -> bool {
        let (kx, kz) = self.key(at);
        let squared = distance * distance;
        (kx - 1..=kx + 1).any(|x| {
            (kz - 1..=kz + 1).any(|z| {
                self.buckets
                    .get(&(x, z))
                    .is_some_and(|list| list.iter().any(|&p| p.distance_squared(at) < squared))
            })
        })
    }
}

/// One Scatter stamp at `center` (world X, Z) over `rect`, whose samples'
/// brush weights are `weights`.
pub(super) fn apply_scatter(
    data: &mut TerrainData,
    placement: &Placement,
    brush: &Brush,
    rect: SampleRect,
    weights: &[f32],
    scatter: &Scatter,
    center: Vec2,
) {
    if let Some(erase) = scatter.erase {
        let drop: Vec<bool> = data
            .foliage()
            .iter()
            .map(|instance| {
                if !instance.in_rect(rect)
                    || (erase == ScatterErase::Palette && !scatter.has(&instance.source))
                {
                    return false;
                }
                let at = placement.to_world_xz(data, Vec2::new(instance.x, instance.z));
                brush.weight(at - center) > 0.0
            })
            .collect();
        if drop.iter().any(|&drop| drop) {
            let mut index = 0;
            data.foliage_mut().retain(|_| {
                let gone = drop[index];
                index += 1;
                !gone
            });
        }
        return;
    }
    // The items placed, each with the running total of weights up to it.
    let mut total = 0.0;
    let picks: Vec<(&ScatterItem, f32)> = scatter
        .items
        .iter()
        .filter(|item| item.places())
        .map(|item| {
            total += item.weight;
            (item, total)
        })
        .collect();
    let density = if scatter.density.is_finite() {
        scatter.density.max(0.0)
    } else {
        0.0
    };
    if picks.is_empty() || density <= 0.0 || data.foliage().len() >= MAX_FOLIAGE_INSTANCES {
        return;
    }
    let (size_x, size_z) = data.size();
    let cell = placement.cell_size(data);
    let cell_area = cell.x * cell.y;

    let spacing = if scatter.spacing.is_finite() {
        scatter.spacing.max(0.0)
    } else {
        0.0
    };
    let mut neighbours = (spacing > 0.0).then(|| {
        let mut neighbours = Neighbours::new(spacing);
        // Painted foliage within the spacing of the stamp's samples.
        let reach_x = (spacing / cell.x.max(1e-6)).ceil();
        let reach_z = (spacing / cell.y.max(1e-6)).ceil();
        let (x0, x1) = (rect.x0 as f32 - reach_x, rect.x1 as f32 + reach_x);
        let (z0, z1) = (rect.z0 as f32 - reach_z, rect.z1 as f32 + reach_z);
        for instance in data.foliage() {
            if instance.x >= x0 && instance.x < x1 && instance.z >= z0 && instance.z < z1 {
                neighbours.add(placement.to_world_xz(data, Vec2::new(instance.x, instance.z)));
            }
        }
        neighbours
    });

    let mut rng = Rng::new(mix(scatter.seed
        ^ u64::from(center.x.to_bits())
        ^ u64::from(center.y.to_bits()).rotate_left(32)));
    let mut added = Vec::new();
    let mut index = 0;
    'samples: for z in rect.z0..rect.z1 {
        for x in rect.x0..rect.x1 {
            let weight = weights[index] * brush.strength;
            index += 1;
            if weight <= 0.0 {
                continue;
            }
            let mut expected = density * cell_area * weight;
            while expected > 0.0 {
                if added.len() >= MAX_FOLIAGE_PER_STAMP {
                    break 'samples;
                }
                let chance = expected.min(1.0);
                expected -= 1.0;
                if rng.next_f32() >= chance {
                    continue;
                }
                let jitter_x = (rng.next_f32() - 0.5) * 0.9;
                let jitter_z = (rng.next_f32() - 0.5) * 0.9;
                let roll = rng.next_f32() * total;
                let turn = rng.next_f32();
                let size = rng.next_f32();
                let gx = (x as f32 + jitter_x).clamp(0.0, (size_x - 1) as f32);
                let gz = (z as f32 + jitter_z).clamp(0.0, (size_z - 1) as f32);
                let at = placement.to_world_xz(data, Vec2::new(gx, gz));
                if brush.weight(at - center) <= 0.0 || data.hole_at(gx, gz) {
                    continue;
                }
                let item = picks
                    .iter()
                    .find(|(_, upto)| roll < *upto)
                    .map_or(picks[picks.len() - 1].0, |(item, _)| item);
                // A masked item is thinned before it takes up room, so the
                // ground its masks leave bare stays free for other items.
                if !item.mask.is_open()
                    && mask_roll(scatter.seed, gx, gz) >= item.mask.density(data, placement, gx, gz)
                {
                    continue;
                }
                if let Some(neighbours) = &mut neighbours {
                    if neighbours.near(at, spacing) {
                        continue;
                    }
                    neighbours.add(at);
                }
                let (low, high) = item.scale_range();
                let rotation = if item.rotation.is_finite() {
                    item.rotation.clamp(0.0, 360.0)
                } else {
                    360.0
                };
                added.push(FoliageInstance {
                    source: item.source.clone(),
                    x: gx,
                    z: gz,
                    yaw: turn * rotation.to_radians(),
                    scale: low + size * (high - low),
                    collide: item.collide,
                    lean: if item.align_to_slope.is_finite() {
                        item.align_to_slope.clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                    sink: if item.sink.is_finite() {
                        item.sink
                    } else {
                        0.0
                    },
                });
            }
        }
    }
    if added.is_empty() {
        return;
    }
    let foliage = data.foliage_mut();
    let room = MAX_FOLIAGE_INSTANCES.saturating_sub(foliage.len());
    foliage.extend(added.into_iter().take(room));
}
