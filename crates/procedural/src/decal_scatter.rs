//! The Scatter pane's decal brush: where one stamp of the brush lays its
//! pictures (stains, tire tracks, patches of grit) as ProjectedDecals, so
//! props sit in some dirt instead of on a clean line. The editor makes the
//! Instances (`editor/src/app/decal_scatter.rs`); this is the layout.
//!
//! Each stamp is deterministic for the stroke's seed and the stamp's
//! middle. A decal's box lies on the surface under its spot, facing the
//! surface's own direction (so it isn't stretched down a slope), turned at
//! random about it, with a size and opacity picked between its picture's
//! two ends.
//!
//! The renderer draws only so many projected decals, the nearest first
//! (`docs/rendering/decals.md`), so a stamp also has a **budget**: it
//! places no decal that would be one too many within the budget's distance
//! of itself, and says how many it left out.

use axcenity_core::Vector3;
use glam::{Quat, Vec2, Vec3};

use crate::orientation_from_rotation;
use crate::rng::Rng;

/// The most decals one stamp adds, so a huge brush or density can't freeze
/// the editor.
pub const MAX_DECALS_PER_STAMP: usize = 64;
/// A scattered decal's box is at least this deep (studs), half above and
/// half below its spot, so it still reaches bumpy ground.
pub const MIN_DEPTH: f32 = 1.0;
/// And at most this deep, so a large one doesn't paint the wall of a
/// building it lies beside from top to bottom.
pub const MAX_DEPTH: f32 = 4.0;
/// A decal's depth as a share of its longer side, between the two above.
pub const DEPTH_SHARE: f32 = 0.35;

/// One picture of the brush's set.
#[derive(Clone, Debug, PartialEq)]
pub struct DecalItem {
    pub texture: String,
    /// Its share of the mix, against the others' weights.
    pub weight: f32,
    /// The smallest and largest its longer side is, in studs.
    pub min_size: f32,
    pub max_size: f32,
    /// Its width over its height.
    pub aspect: f32,
    /// The faintest and strongest it's drawn, 0 to 1.
    pub min_opacity: f32,
    pub max_opacity: f32,
    /// Degrees of random turn on the surface, 0 to 360.
    pub rotation: f32,
}

/// The decal brush.
#[derive(Clone, Debug, PartialEq)]
pub struct DecalBrush {
    pub items: Vec<DecalItem>,
    /// Studs from its middle to its rim.
    pub radius: f32,
    /// 0 to 1: how much of the radius thins out toward the rim.
    pub falloff: f32,
    /// Decals per square stud at full strength.
    pub density: f32,
    /// Studs kept between a new decal's middle and any other's (0: none).
    pub spacing: f32,
    /// Where the stroke's random numbers start.
    pub seed: u64,
}

/// How many projected decals may lie within `distance` studs of one
/// another: a quality preset's limits (`render::Quality::decal_limits`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecalBudget {
    pub count: usize,
    pub distance: f32,
}

/// A decal to make: a ProjectedDecal's properties.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedDecal {
    pub texture: String,
    pub position: Vec3,
    /// Degrees, as the `Orientation` property.
    pub orientation: Vector3,
    /// The picture's width and height, then the box's depth.
    pub size: Vec3,
    /// 0 (solid) to 1 (invisible): 1 less its opacity.
    pub transparency: f32,
}

/// What one stamp did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stamped {
    pub placed: Vec<PlacedDecal>,
    /// Decals it would have placed but for the budget.
    pub over_budget: usize,
}

/// How strongly the brush acts `away` studs from its middle: 1 inside,
/// fading to 0 at the rim over `falloff` of the radius.
fn strength(brush: &DecalBrush, away: f32) -> f32 {
    let radius = brush.radius.max(0.01);
    if away >= radius {
        return 0.0;
    }
    let inner = radius * (1.0 - brush.falloff.clamp(0.0, 1.0));
    if away <= inner {
        1.0
    } else {
        1.0 - (away - inner) / (radius - inner)
    }
}

/// The `Orientation` of a decal lying on a surface facing `normal`, turned
/// `turn` radians about it: its front (-Z, the way its picture faces) is
/// the normal.
pub fn lying_on(normal: Vec3, turn: f32) -> Vector3 {
    let normal = normal.normalize_or(Vec3::Y);
    orientation_from_rotation(
        Quat::from_rotation_arc(Vec3::NEG_Z, normal) * Quat::from_rotation_z(turn),
    )
}

/// One stamp of `brush` with its middle at `center`: the decals to make.
///
/// `surface` gives the point on the surface under a spot (world X and Z)
/// and the surface's direction there, or `None` where there's nothing to
/// lie on. `existing` is where every projected decal in the world already
/// is (the stamp's own are added to it as it goes): new ones keep the
/// brush's spacing from them, and none is placed where `budget` decals
/// already lie within its distance.
pub fn stamp(
    brush: &DecalBrush,
    center: Vec3,
    existing: &mut Vec<Vec3>,
    budget: DecalBudget,
    surface: impl Fn(Vec2) -> Option<(Vec3, Vec3)>,
) -> Stamped {
    let mut stamped = Stamped::default();
    // The pictures placed, each with the running total of weights up to it.
    let mut total = 0.0;
    let picks: Vec<(&DecalItem, f32)> = brush
        .items
        .iter()
        .filter(|item| item.weight.is_finite() && item.weight > 0.0 && !item.texture.is_empty())
        .map(|item| {
            total += item.weight;
            (item, total)
        })
        .collect();
    let density = if brush.density.is_finite() {
        brush.density.max(0.0)
    } else {
        0.0
    };
    let radius = brush.radius.max(0.01);
    if picks.is_empty() || density <= 0.0 {
        return stamped;
    }
    let spacing = if brush.spacing.is_finite() {
        brush.spacing.max(0.0)
    } else {
        0.0
    };
    let seed =
        brush.seed ^ u64::from(center.x.to_bits()) ^ u64::from(center.z.to_bits()).rotate_left(32);
    // How many the whole circle gets at full strength; the rim's falloff
    // then leaves some of them out.
    let expected = density * std::f32::consts::PI * radius * radius;
    let tries = (expected.floor() as usize
        + usize::from(Rng::new(seed, u64::MAX).unit() < expected.fract()))
    .min(MAX_DECALS_PER_STAMP * 4);
    for index in 0..tries {
        if stamped.placed.len() >= MAX_DECALS_PER_STAMP {
            break;
        }
        // Each try has its own numbers, drawn in a fixed order.
        let mut rng = Rng::new(seed, index as u64);
        let (away, angle) = (
            rng.unit().sqrt() * radius,
            rng.unit() * std::f32::consts::TAU,
        );
        let keep = rng.unit();
        let roll = rng.unit() * total;
        let (turn, size, opacity) = (rng.unit(), rng.unit(), rng.unit());
        if keep >= strength(brush, away) {
            continue;
        }
        let spot = Vec2::new(center.x + angle.cos() * away, center.z + angle.sin() * away);
        let Some((point, normal)) = surface(spot) else {
            continue;
        };
        if spacing > 0.0
            && existing
                .iter()
                .any(|other| other.distance_squared(point) < spacing * spacing)
        {
            continue;
        }
        let nearby = existing
            .iter()
            .filter(|other| other.distance_squared(point) <= budget.distance * budget.distance)
            .count();
        if nearby >= budget.count {
            stamped.over_budget += 1;
            continue;
        }
        let item = picks
            .iter()
            .find(|(_, upto)| roll < *upto)
            .map_or(picks[picks.len() - 1].0, |(item, _)| item);
        let range = |a: f32, b: f32, t: f32, fallback: f32| {
            let (a, b) = (
                if a.is_finite() { a } else { fallback },
                if b.is_finite() { b } else { fallback },
            );
            a.min(b) + (a.max(b) - a.min(b)) * t
        };
        let longer = range(item.min_size, item.max_size, size, 4.0).max(0.01);
        let aspect = if item.aspect.is_finite() && item.aspect > 0.0 {
            item.aspect
        } else {
            1.0
        };
        let (width, height) = if aspect >= 1.0 {
            (longer, longer / aspect)
        } else {
            (longer * aspect, longer)
        };
        let rotation = if item.rotation.is_finite() {
            item.rotation.clamp(0.0, 360.0)
        } else {
            360.0
        };
        existing.push(point);
        stamped.placed.push(PlacedDecal {
            texture: item.texture.clone(),
            position: point,
            orientation: lying_on(normal, turn * rotation.to_radians()),
            size: Vec3::new(
                width,
                height,
                (longer * DEPTH_SHARE).clamp(MIN_DEPTH, MAX_DEPTH),
            ),
            transparency: 1.0
                - range(item.min_opacity, item.max_opacity, opacity, 1.0).clamp(0.0, 1.0),
        });
    }
    stamped
}

#[cfg(test)]
#[path = "decal_scatter_tests.rs"]
mod tests;
