//! Roads and paths on the terrain (procedural worlds PW4). A `TerrainPath`
//! inside a Spline says how wide the road is and what it does to the
//! ground: scatter rules keep off its strip, live, while it's there
//! ([`keep_offs`], read by [`terrain_scatter`](crate::terrain_scatter)),
//! and [`apply_path`] flattens and paints the ground under it (the editor's
//! Apply, an undo step, since heights and paint are saved).
//!
//! So a road doesn't end in a clean line, Apply can lay a second layer (a
//! gravel or dust shoulder) from the road's edge out over its `Falloff`,
//! break the paint's outer edge up with noise (`EdgeNoise`), and take the
//! painted foliage off the strip, thinning it over `ClearFalloff` past it;
//! scatter rules thin out over the same distance, live.
//!
//! Distances are seen from above: a strip is every point within half the
//! path's width of the curve, however the curve climbs.

use axcenity_core::spline::SplineCurve;
use axcenity_core::{ClassName, Instance, InstanceId, SampleRect, TerrainData, Value, World};
use axcenity_terrain::Placement;
use axcenity_terrain::brush::{Operation, PaintMode, paint_sample};
use axcenity_terrain::noise::{Noise, mix, smoothstep};
use glam::{Vec2, Vec3};

use crate::flatten::nearest;
use crate::terrain_scatter::fnv;
use crate::v3;

/// Studs between the points a keep-off strip is followed by.
const KEEP_OFF_SPACING: f32 = 2.0;
/// With a shoulder layer, the road's own layer blends into it over this
/// share of the falloff, so the shoulder shows beside the road.
const ROAD_EDGE: f32 = 0.25;
/// The seed of the noise that breaks a path's painted edge up. It's by
/// world position, so two paths (or two stretches of one) differ anyway.
const EDGE_NOISE_SEED: u64 = 0x0ED6_E5EE_D000_0001;

/// A TerrainPath's settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PathSettings {
    pub width: f32,
    pub falloff: f32,
    pub flatten: bool,
    /// The TerrainLayer it paints, by name; empty for none.
    pub layer: String,
    /// The TerrainLayer its shoulders are painted with, from each edge out
    /// over `falloff`, by name; empty for none.
    pub shoulder_layer: String,
    /// 0 to 1: how much noise breaks the paint's outer edge up. The
    /// distance the paint reaches past the road's edge is `falloff` times
    /// 1 - `edge_noise` to 1 + `edge_noise`, from place to place.
    pub edge_noise: f32,
    /// Studs across the edge noise's patches.
    pub noise_scale: f32,
    pub clear_scatter: bool,
    pub clear_margin: f32,
    /// Studs past the cleared strip over which scatter thins out, from
    /// none at the strip to all of it.
    pub clear_falloff: f32,
    /// Route's steepest climb, in degrees.
    pub max_grade: f32,
}

impl PathSettings {
    /// A TerrainPath's settings, or `None` for another class or one that's
    /// switched off.
    pub fn of(instance: &Instance) -> Option<PathSettings> {
        if instance.class() != ClassName::TerrainPath
            || instance.property("Enabled") != Some(&Value::Bool(true))
        {
            return None;
        }
        let number = |name, default: f32| {
            instance
                .property(name)
                .and_then(Value::as_number)
                .map(|n| n as f32)
                .filter(|n| n.is_finite())
                .unwrap_or(default)
        };
        let on = |name| instance.property(name) == Some(&Value::Bool(true));
        let text = |name| {
            instance
                .property(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        Some(PathSettings {
            width: number("Width", 8.0).max(0.0),
            falloff: number("Falloff", 4.0).max(0.0),
            flatten: on("Flatten"),
            layer: text("Layer"),
            shoulder_layer: text("ShoulderLayer"),
            edge_noise: number("EdgeNoise", 0.5).clamp(0.0, 1.0),
            noise_scale: number("NoiseScale", 10.0).max(0.5),
            clear_scatter: on("ClearScatter"),
            clear_margin: number("ClearMargin", 1.0).max(0.0),
            clear_falloff: number("ClearFalloff", 4.0).max(0.0),
            max_grade: number("MaxGrade", 8.0).clamp(0.5, 60.0),
        })
    }
}

impl PathSettings {
    /// The most studs from the curve its paint reaches: half the width,
    /// then the falloff at its widest (where the edge noise is strongest).
    pub fn paint_reach(&self) -> f32 {
        self.width / 2.0 + self.falloff * (1.0 + self.edge_noise)
    }

    /// How far the paint reaches past the road's edge at world `at` (X,
    /// Z), as a share of the falloff: 1 without edge noise, else from
    /// 1 - `edge_noise` to 1 + `edge_noise`, changing smoothly from place
    /// to place.
    pub fn edge_reach(&self, at: Vec2) -> f32 {
        if self.edge_noise <= 0.0 {
            return 1.0;
        }
        let scale = self.noise_scale.max(0.5);
        // Fractal noise stays mostly within ±0.6.
        let n = (Noise::new(EDGE_NOISE_SEED).fbm(at.x / scale, at.y / scale, 3, 0.5, 2.0) / 0.6)
            .clamp(-1.0, 1.0);
        1.0 + n * self.edge_noise
    }

    /// How much of each layer Apply paints `away` studs from the curve at
    /// world `at` (X, Z), each 0 to 1: the path's own `Layer`, then its
    /// `ShoulderLayer`. Both are full on the road (within half its width).
    /// Past its edge the shoulder fades out over the falloff (times
    /// [`edge_reach`](Self::edge_reach), so its outer edge is ragged). The
    /// road's own layer does the same without a shoulder; with one
    /// (`shoulder`), it blends into it over the first quarter of that, so
    /// the shoulder shows beside it.
    pub fn paint_amounts(&self, away: f32, at: Vec2, shoulder: bool) -> (f32, f32) {
        let half = self.width / 2.0;
        if away <= half {
            return (1.0, 1.0);
        }
        let band = self.falloff * self.edge_reach(at);
        let fade = |band: f32| 1.0 - smoothstep(half, half + band, away);
        if shoulder {
            (fade(band * ROAD_EDGE), fade(band))
        } else {
            (fade(band), 0.0)
        }
    }

    /// 0 to 1: how much of the scatter `away` studs from the curve is
    /// kept: none within half the width and the clear margin, then more
    /// and more over the clear falloff, and all of it past that.
    pub fn scatter_kept(&self, away: f32) -> f32 {
        let reach = self.width / 2.0 + self.clear_margin;
        if away <= reach {
            0.0
        } else {
            smoothstep(reach, reach + self.clear_falloff, away)
        }
    }
}

/// A number from 0 up to (not including) 1 for a spot (X and Z, in any
/// units), the same on every device: what a spot's chance of being kept
/// in a thinning strip is rolled against.
pub(crate) fn spot_roll(at: Vec2) -> f32 {
    let hash = mix(0x7A11_0FF5_0000_0000
        ^ u64::from(at.x.to_bits())
        ^ u64::from(at.y.to_bits()).rotate_left(32));
    (hash >> 40) as f32 / (1u64 << 24) as f32
}

/// A strip scatter rules keep off: within `reach` studs of `line`, seen
/// from above (world X and Z), thinning out over `fade` studs past that.
#[derive(Clone, Debug, PartialEq)]
pub struct KeepOff {
    pub line: Vec<Vec2>,
    pub reach: f32,
    /// Studs past `reach` over which rules place fewer and fewer.
    pub fade: f32,
    /// The strip's box.
    pub min: Vec2,
    pub max: Vec2,
    /// Of the line and reach, to tell when the strip changed.
    pub hash: u64,
}

impl KeepOff {
    /// The strip within `reach` studs of `line`, or `None` without points.
    pub fn new(line: Vec<Vec2>, reach: f32) -> Option<KeepOff> {
        Self::with_fade(line, reach, 0.0)
    }

    /// [`new`](Self::new), thinning out over `fade` studs past `reach`.
    pub fn with_fade(line: Vec<Vec2>, reach: f32, fade: f32) -> Option<KeepOff> {
        if line.is_empty() {
            return None;
        }
        let fade = fade.max(0.0);
        let (min, max) = line
            .iter()
            .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(min, max), &p| {
                (min.min(p), max.max(p))
            });
        let mut bytes = Vec::with_capacity(line.len() * 8 + 4);
        for p in &line {
            bytes.extend(p.x.to_bits().to_le_bytes());
            bytes.extend(p.y.to_bits().to_le_bytes());
        }
        bytes.extend(reach.to_bits().to_le_bytes());
        // Left out without a fade, so a strip that doesn't thin keeps the
        // hash it always had.
        if fade > 0.0 {
            bytes.extend(fade.to_bits().to_le_bytes());
        }
        Some(KeepOff {
            hash: fnv(&bytes),
            min: min - Vec2::splat(reach + fade),
            max: max + Vec2::splat(reach + fade),
            line,
            reach,
            fade,
        })
    }

    /// How far world `at` (X, Z) is from the line, squared.
    fn distance_squared(&self, at: Vec2) -> f32 {
        if self.line.len() == 1 {
            return self.line[0].distance_squared(at);
        }
        self.line
            .windows(2)
            .map(|pair| segment_distance_squared(pair[0], pair[1], at))
            .fold(f32::INFINITY, f32::min)
    }

    /// Whether a rule's spot at world `at` (X, Z) is left out: always
    /// inside the strip, and past it over the fade more and more are kept
    /// (each spot by a number of its own, so the same ones on every
    /// device).
    pub fn clears(&self, at: Vec2) -> bool {
        if at.x < self.min.x || at.y < self.min.y || at.x > self.max.x || at.y > self.max.y {
            return false;
        }
        let away = self.distance_squared(at).sqrt();
        if away <= self.reach {
            return true;
        }
        if self.fade <= 0.0 || away >= self.reach + self.fade {
            return false;
        }
        spot_roll(at) >= smoothstep(self.reach, self.reach + self.fade, away)
    }

    /// Whether world `at` (X, Z) is inside the strip (not its fade).
    pub fn covers(&self, at: Vec2) -> bool {
        if at.x < self.min.x || at.y < self.min.y || at.x > self.max.x || at.y > self.max.y {
            return false;
        }
        let reach = self.reach * self.reach;
        if self.line.len() == 1 {
            return self.line[0].distance_squared(at) <= reach;
        }
        self.line
            .windows(2)
            .any(|pair| segment_distance_squared(pair[0], pair[1], at) <= reach)
    }

    /// Whether the strip's box overlaps the box from `min` to `max`.
    pub fn overlaps(&self, min: Vec2, max: Vec2) -> bool {
        self.min.x <= max.x && self.min.y <= max.y && self.max.x >= min.x && self.max.y >= min.y
    }
}

fn segment_distance_squared(a: Vec2, b: Vec2, at: Vec2) -> f32 {
    let along = b - a;
    let t = if along.length_squared() > 1e-12 {
        ((at - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + along * t).distance_squared(at)
}

/// The Spline a TerrainPath is in, if its parent is one.
pub fn spline_of(world: &World, path: InstanceId) -> Option<InstanceId> {
    let parent = world.get(path)?.parent()?;
    (world.get(parent)?.class() == ClassName::Spline).then_some(parent)
}

/// The strips scatter rules keep off: every TerrainPath in a Spline that's
/// on and has `ClearScatter`, in tree order, `Width / 2 + ClearMargin`
/// studs either side of its curve, thinning out over `ClearFalloff` past
/// that.
pub fn keep_offs(world: &World) -> Vec<KeepOff> {
    world
        .all_below(world.root(), |class| class == ClassName::TerrainPath)
        .into_iter()
        .filter_map(|path| {
            let settings = PathSettings::of(world.get(path)?)?;
            if !settings.clear_scatter {
                return None;
            }
            let curve = SplineCurve::of(world, spline_of(world, path)?)?;
            let line = curve
                .polyline(KEEP_OFF_SPACING)
                .into_iter()
                .map(|p| Vec2::new(p.x, p.z))
                .collect();
            KeepOff::with_fade(
                line,
                settings.width / 2.0 + settings.clear_margin,
                settings.clear_falloff,
            )
        })
        .collect()
}

/// What [`apply_path`] changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Terrain samples flattened or painted.
    pub samples: usize,
    /// Painted foliage taken away.
    pub foliage: usize,
}

impl Applied {
    /// Whether anything changed.
    pub fn any(&self) -> bool {
        self.samples > 0 || self.foliage > 0
    }
}

/// Flattens and paints `data` under `curve` as `path` says, and clears its
/// painted foliage:
///
/// - **Heights** (with `Flatten`): within half its width the ground is put
///   at the height of the curve's nearest point, blending back into the
///   ground over its `Falloff`. Never ragged, so the road stays level.
/// - **Paint**: `layer` (the path's `Layer`) and `shoulder` (its
///   `ShoulderLayer`), each the layer's place among the terrain's layers
///   or `None` to paint nothing, by [`PathSettings::paint_amounts`]: the
///   shoulder first, then the road's layer over it.
/// - **Painted foliage** (with `ClearScatter`): taken away within half the
///   width and `ClearMargin`, and thinned over `ClearFalloff` past that
///   ([`PathSettings::scatter_kept`]; which go is by each one's place, so
///   applying again takes no more).
pub fn apply_path(
    data: &mut TerrainData,
    placement: &Placement,
    curve: &SplineCurve,
    path: &PathSettings,
    layer: Option<usize>,
    shoulder: Option<usize>,
) -> Applied {
    let paints = layer.is_some() || shoulder.is_some();
    let clears = path.clear_scatter && !data.foliage().is_empty();
    if !path.flatten && !paints && !clears {
        return Applied::default();
    }
    let cell = placement.cell_size(data).min_element().max(1e-3);
    let line: Vec<Vec3> = curve
        .polyline((cell / 2.0).max(0.25))
        .into_iter()
        .map(v3)
        .collect();
    let mut applied = Applied::default();
    if clears {
        applied.foliage = clear_foliage(data, placement, &line, path);
    }
    if !path.flatten && !paints {
        return applied;
    }
    let half = path.width / 2.0;
    let level_reach = half + path.falloff;
    let reach = if paints {
        level_reach.max(path.paint_reach())
    } else {
        level_reach
    };
    let Some(rect) = rect_around(data, placement, &line, reach) else {
        return applied;
    };
    let (size_x, _) = data.size();
    let paint = |layer| Operation::Paint {
        layer,
        mode: PaintMode::Blend,
        locked: [false; axcenity_core::MAX_TERRAIN_LAYERS],
    };
    let (road, verge) = (layer.map(paint), shoulder.map(paint));
    // Worked out first, so the heights aren't borrowed while reading.
    let mut set = Vec::new();
    for z in rect.z0..rect.z1 {
        for x in rect.x0..rect.x1 {
            let at = placement.to_world_xz(data, Vec2::new(x as f32, z as f32));
            let Some((away, y)) = nearest(&line, at) else {
                continue;
            };
            if away > reach {
                continue;
            }
            let index = z * size_x + x;
            let was = (data.heights()[index], data.paint()[index]);
            let mut height = was.0;
            if path.flatten && away <= level_reach {
                let amount = if away <= half {
                    1.0
                } else {
                    1.0 - smoothstep(half, level_reach, away)
                };
                height += (placement.height_of(y) - height) * amount;
            }
            let mut weights = was.1;
            if paints {
                let (road_amount, verge_amount) = path.paint_amounts(away, at, verge.is_some());
                if let Some(operation) = &verge
                    && verge_amount > 0.0
                {
                    weights = paint_sample(weights, operation, verge_amount);
                }
                if let Some(operation) = &road
                    && road_amount > 0.0
                {
                    weights = paint_sample(weights, operation, road_amount);
                }
            }
            // Without edge noise every sample within reach counts, as it
            // always did; with it, only those the ragged edge reaches.
            if path.edge_noise <= 0.0 || away <= level_reach || (height, weights) != was {
                set.push((index, height, weights));
            }
        }
    }
    if set.is_empty() {
        return applied;
    }
    let (heights, paints) = data.both_mut_in(rect);
    for &(index, height, weights) in &set {
        heights[index] = height;
        paints[index] = weights;
    }
    data.clamp_heights_in(rect);
    applied.samples = set.len();
    applied
}

/// Takes `data`'s painted foliage off `path`'s strip along `line`, thinning
/// it over the clear falloff. Returns how many it took.
fn clear_foliage(
    data: &mut TerrainData,
    placement: &Placement,
    line: &[Vec3],
    path: &PathSettings,
) -> usize {
    if line.is_empty() {
        return 0;
    }
    let reach = path.width / 2.0 + path.clear_margin + path.clear_falloff;
    let (min, max) = line
        .iter()
        .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(min, max), p| {
            let p = Vec2::new(p.x, p.z);
            (min.min(p), max.max(p))
        });
    let (min, max) = (min - Vec2::splat(reach), max + Vec2::splat(reach));
    let gone: Vec<bool> = data
        .foliage()
        .iter()
        .map(|instance| {
            let spot = Vec2::new(instance.x, instance.z);
            let at = placement.to_world_xz(data, spot);
            if at.x < min.x || at.y < min.y || at.x > max.x || at.y > max.y {
                return false;
            }
            let Some((away, _)) = nearest(line, at) else {
                return false;
            };
            // By its place in the terrain's samples, which a moved or
            // resized Terrain keeps.
            away < reach && spot_roll(spot) >= path.scatter_kept(away)
        })
        .collect();
    let taken = gone.iter().filter(|&&gone| gone).count();
    if taken > 0 {
        let mut index = 0;
        data.foliage_mut().retain(|_| {
            let keep = !gone[index];
            index += 1;
            keep
        });
    }
    taken
}

/// The samples of `data` within `reach` studs of `line`'s box, or `None`
/// when that's off the terrain.
pub(crate) fn rect_around(
    data: &TerrainData,
    placement: &Placement,
    line: &[Vec3],
    reach: f32,
) -> Option<SampleRect> {
    if line.is_empty() {
        return None;
    }
    let (min, max) = line
        .iter()
        .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(min, max), p| {
            let p = Vec2::new(p.x, p.z);
            (min.min(p), max.max(p))
        });
    let (size_x, size_z) = data.size();
    let low = placement
        .to_grid(data, min.x - reach, min.y - reach)
        .floor();
    let high = placement.to_grid(data, max.x + reach, max.y + reach).ceil();
    let clamp_x = |v: f32| (v.max(0.0) as usize).min(size_x);
    let clamp_z = |v: f32| (v.max(0.0) as usize).min(size_z);
    let rect = SampleRect::new(
        clamp_x(low.x),
        clamp_z(low.y),
        clamp_x(high.x + 1.0),
        clamp_z(high.y + 1.0),
    );
    (!rect.is_empty()).then_some(rect)
}
