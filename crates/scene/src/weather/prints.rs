//! What's pressed into the snow: footprints, ruts and the hollows things
//! lie in.
//!
//! Each update, [`Prints`] finds what touches the snow near the camera: each
//! foot of a character, and each Part or MeshPart that collides and has
//! moved (or lies unanchored in the snow). For each it finds the ground
//! under the thing with a ray that passes the thing itself by, the snow's
//! surface over that ground, and the part of the thing's own shape under
//! that surface ([`Contact`]): a convex outline seen from above, and how far
//! its lowest point is into the snow as a share of the snow's depth.
//!
//! That outline, swept from where it was last update, is stamped into a fine
//! grid round the camera ([`Map`], [`WeatherPrintCells`]: an eighth of a stud
//! a cell) with a soft wall, deepest in its middle, and a rim of pushed-out
//! snow round a deep one. The shaders read it (lighting.wgsl's
//! `weather_print`). The outlines are kept too ([`Stamp`]), so prints the
//! camera leaves behind are stamped again when it comes back.
//!
//! Prints fill in at one rate (a whole depth each `FootprintFadeTime`,
//! faster while it snows). A cell holds its depth plus how far everything
//! had filled in when it was stamped ([`Prints::fill`]), so the cells don't
//! change as prints fill in: the shader takes the filling since away.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axcenity_core::{ClassName, Instance, InstanceId, PartType, Value, Vector3, World};
use axcenity_render::glam::{Quat, Vec2, Vec3};
use axcenity_render::{
    WEATHER_PRINT_CELLS_PER_STUD, WEATHER_PRINT_LEVELS, WEATHER_PRINT_MOST, WEATHER_PRINT_SIZE,
    WeatherPrintCells, WeatherPrints,
};

use crate::orientation::rotation_from_orientation;
use crate::particles::Surfaces;

/// The map's side in cells and in studs, and its cells to a stud.
const SIDE: i32 = WEATHER_PRINT_SIZE as i32;
const CELLS: f32 = WEATHER_PRINT_CELLS_PER_STUD;
const STUDS: i32 = SIDE / CELLS as i32;
/// The map follows the camera in steps of this many studs, so it's moved
/// (and uploaded whole) only that often.
const SNAP: i32 = 8;
/// Prints fade out over this many studs from the map's edge. Must match
/// `PRINT_FADE` in lighting.wgsl.
const EDGE_FADE: f32 = 4.0;
/// Things further from the camera than this (studs, level) press nothing.
const RANGE: f32 = 48.0;
/// The most outlines kept at once; the oldest go first.
const MOST_STAMPS: usize = 4096;
/// The most things whose ground is found in one update (a ray each, or two
/// for a character's feet; each is a query of the whole Workspace). The
/// rest wait for the next. The ground found under a thing is kept while
/// the thing stays within [`GROUND_KEEPS`] studs (level) of where it was
/// found: the plane it lies in stands in for it that far.
const GROUNDS_AN_UPDATE: usize = 32;
const GROUND_KEEPS: f32 = 0.75;
/// How many parts an update looks at for ones that lie in the snow without
/// moving, or that the camera has left behind.
const LOOKS_AN_UPDATE: usize = 256;
/// The list of the Workspace's parts is read again at most this often
/// (seconds) while parts come and go.
const PARTS_EVERY: f32 = 0.25;
/// How wide a print's wall is, in studs, where its shape allows: the snow
/// slopes from the outline's edge down to the print's floor over it.
const WALL: f32 = 0.15;
/// Snow pushed out of a print stands in a rim this wide round it, in
/// studs, from prints this deep (a share of the snow's depth: none below
/// the first, whole from the second).
const RIM_WIDTH: f32 = 0.3;
const RIM_FROM: (f32, f32) = (0.3, 0.8);
/// Snow counts as at least this deep (studs), so a dusting still takes the
/// print of what stands on the ground.
const THIN: f32 = 0.03;
/// A thing that sinks in less than this share of the snow's depth leaves
/// nothing.
const LIGHTEST: f32 = 0.03;
/// A thing that moved further than this in one update (studs) was put
/// there, not dragged: no trail between.
const SWEEP_MOST: f32 = 6.0;
/// One kept outline covers at most this many studs of a trail, and a trail
/// that leaves the straight line by this many studs, or turns its thing by
/// more than about 8 degrees, starts another.
const STAMP_LONGEST: f32 = 3.0;
const STAMP_STRAIGHT: f32 = 0.08;
const STAMP_TURN: f32 = 0.99;
/// The print under a thing that lies still is stamped again each time it has
/// filled in by this share of its depth.
const REST_AGAIN: f32 = 0.25;
/// When everything has filled in by this many whole depths the cells are
/// counted down, so their 12 bits never run out.
const REBASE: f32 = 2.5;

/// The next [`WeatherPrintCells::key`].
static KEYS: AtomicU64 = AtomicU64::new(1);

/// What the prints need to know of the weather now.
pub(super) struct Snow<'a> {
    /// How much snow lies (`Weather.SnowCover`, eased) and how thick
    /// (`SnowThickness`), how hard it falls, and `FootprintFadeTime`.
    pub cover: f32,
    pub thickness: f32,
    pub falling: f32,
    pub fade: f32,
    /// The share of the world's snow lying on the ground at a point, under
    /// a thing whose top is at the given height: 0 under a roof, up to 2 in
    /// a WeatherVolume.
    pub share: &'a dyn Fn(Vec3, f32) -> f32,
}

impl Snow<'_> {
    /// How deep the snow lies at its whole share, in studs: the thickness,
    /// eased in with the cover as the shaders ease it (`vs_terrain`,
    /// `snow_cap`). The long drifts (`snow_drift`, 0.7 to 1.3 of it on
    /// terrain) and the 0.7 of it that stays on parts aren't counted.
    fn depth(&self) -> f32 {
        let t = ((self.cover - 0.2) / 0.7).clamp(0.0, 1.0);
        self.thickness.max(0.0) * t * t * (3.0 - 2.0 * t)
    }
}

/// A convex outline seen from above (x and z), grown by `radius`, pressed
/// `depth` of the snow's depth down.
#[derive(Clone, Debug, PartialEq)]
struct Shape {
    /// Its corners, counter-clockwise: one for a disc, two for a capsule.
    points: Vec<Vec2>,
    radius: f32,
    /// The studs from its edge over which it reaches its whole depth.
    soft: f32,
    depth: f32,
}

impl Shape {
    /// How high the rim round it stands, as a share of the most.
    fn rim(&self) -> f32 {
        let t = ((self.depth - RIM_FROM.0) / (RIM_FROM.1 - RIM_FROM.0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// How long it lasts, in whole depths filled in.
    fn life(&self) -> f32 {
        self.depth.max(self.rim())
    }

    /// Its sides, each from a corner, with one over its squared length:
    /// worked out once for the many cells a stamp asks about
    /// ([`sides_distance`]).
    fn sides(&self) -> Vec<(Vec2, Vec2, f32)> {
        let count = self.points.len();
        (0..if count > 2 {
            count
        } else {
            count.saturating_sub(1)
        })
            .map(|i| {
                let along = self.points[(i + 1) % count] - self.points[i];
                let length = along.length_squared();
                let over = if length > 0.0 { 1.0 / length } else { 0.0 };
                (self.points[i], along, over)
            })
            .collect()
    }

    /// The box it and its rim lie in.
    fn bounds(&self) -> (Vec2, Vec2) {
        let reach = self.radius + if self.rim() > 0.0 { RIM_WIDTH } else { 0.0 };
        let low = self.points.iter().copied().reduce(Vec2::min);
        let high = self.points.iter().copied().reduce(Vec2::max);
        (
            low.unwrap_or(Vec2::ZERO) - reach,
            high.unwrap_or(Vec2::ZERO) + reach,
        )
    }

    /// The outline round this one and `other`: what a thing sweeps going
    /// straight from one to the other.
    fn joined(&self, other: &Shape) -> Shape {
        let mut points = self.points.clone();
        points.extend_from_slice(&other.points);
        Shape {
            points: hull(points),
            radius: self.radius.max(other.radius),
            soft: self.soft.min(other.soft),
            depth: self.depth.max(other.depth),
        }
    }
}

/// The studs from `p` to the convex outline `points` (counter-clockwise):
/// negative inside it.
fn outline_distance(points: &[Vec2], p: Vec2) -> f32 {
    match points {
        [] => f32::INFINITY,
        [a] => (p - *a).length(),
        [a, b] => to_segment(p, *a, *b).sqrt(),
        _ => {
            let mut nearest = f32::INFINITY;
            let mut inside = true;
            for (i, &a) in points.iter().enumerate() {
                let b = points[(i + 1) % points.len()];
                nearest = nearest.min(to_segment(p, a, b));
                inside &= (b - a).perp_dot(p - a) >= 0.0;
            }
            if inside {
                -nearest.sqrt()
            } else {
                nearest.sqrt()
            }
        }
    }
}

/// [`outline_distance`] for an outline given by its [`Shape::sides`] (or by
/// its one point, when it has no side).
fn sides_distance(sides: &[(Vec2, Vec2, f32)], point: Option<Vec2>, p: Vec2) -> f32 {
    let mut nearest = f32::INFINITY;
    let mut inside = sides.len() > 2;
    for &(from, along, over) in sides {
        let to = p - from;
        let t = (to.dot(along) * over).clamp(0.0, 1.0);
        nearest = nearest.min((to - along * t).length_squared());
        inside &= along.perp_dot(to) >= 0.0;
    }
    if sides.is_empty() {
        return point.map_or(f32::INFINITY, |point| point.distance(p));
    }
    if inside {
        -nearest.sqrt()
    } else {
        nearest.sqrt()
    }
}

/// The squared distance from `p` to the segment from `a` to `b`.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length = along.length_squared();
    let t = if length > 0.0 {
        ((p - a).dot(along) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - a - along * t).length_squared()
}

/// The convex outline round `points`, counter-clockwise, with no point
/// twice and none on a straight side (Andrew's monotone chain).
fn hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.retain(|p| p.is_finite());
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup_by(|a, b| a.distance_squared(*b) < 1e-8);
    if points.len() < 3 {
        return points;
    }
    let mut out: Vec<Vec2> = Vec::with_capacity(points.len() + 1);
    let turn = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    for pass in 0..2 {
        let start = out.len();
        let add = |p: Vec2, out: &mut Vec<Vec2>| {
            while out.len() >= start + 2 && turn(out[out.len() - 2], out[out.len() - 1], p) <= 1e-7
            {
                out.pop();
            }
            out.push(p);
        };
        if pass == 0 {
            for &p in &points {
                add(p, &mut out);
            }
        } else {
            for &p in points.iter().rev() {
                add(p, &mut out);
            }
        }
        // The last point of each pass starts the next.
        out.pop();
    }
    if out.len() < 3 {
        // All on one line: its two ends.
        return vec![points[0], points[points.len() - 1]];
    }
    out
}

/// The snow's surface over the ground under a thing: a plane through
/// `point` (the ground under the thing, lifted by the snow's depth there)
/// that lies as the ground does.
#[derive(Clone, Copy, Debug)]
struct Plane {
    point: Vec3,
    normal: Vec3,
    /// How deep the snow lies there, in studs.
    deep: f32,
}

impl Plane {
    /// How far `q` is over the snow's surface, straight up: negative in it.
    fn over(&self, q: Vec3) -> f32 {
        (q - self.point).dot(self.normal) / self.normal.y
    }
}

/// The ground under a thing: where a ray down through it lands, passing
/// the thing itself by, and how the ground lies there.
#[derive(Clone, Copy, Debug)]
struct Ground {
    point: Vec3,
    normal: Vec3,
}

/// The ground found under a thing's low point when that was at `at`
/// (level) and `low` high: `None` if there was none in reach.
#[derive(Clone, Copy, Debug)]
struct Found {
    at: Vec2,
    low: f32,
    ground: Option<Ground>,
}

impl Found {
    /// Whether it still stands for the ground under a thing whose low
    /// point is now at `low`. No ground in reach is only as true as the
    /// thing is high: one that has dropped a quarter stud is looked under
    /// again.
    fn holds(&self, low: Vec3) -> bool {
        self.at.distance(level(low)) <= GROUND_KEEPS
            && (self.ground.is_some() || low.y >= self.low - 0.25)
    }
}

/// The snow's surface over `ground` for a thing whose top is at `top`, or
/// `None` where no snow lies (under shelter, or too steep).
fn snow_over(snow: &Snow, ground: Ground, top: f32) -> Option<Plane> {
    // As steep ground holds no snow in the shaders (`snow_lying`).
    if 1.5 * snow.cover - 2.5 * (1.0 - ground.normal.y) <= 0.1 {
        return None;
    }
    let share = (snow.share)(ground.point, top);
    if share.is_nan() || share <= 0.02 {
        return None;
    }
    let deep = (snow.depth() * share.min(2.0)).max(THIN);
    Some(Plane {
        point: ground.point + Vec3::Y * deep,
        // A wall's normal would make a plane nothing rests on.
        normal: if ground.normal.y >= 0.6 {
            ground.normal
        } else {
            Vec3::Y
        },
        deep,
    })
}

/// Where a thing meets the snow now.
#[derive(Clone, Debug)]
struct Contact {
    shape: Shape,
    /// Its middle, and the way the thing is turned (level, a unit long).
    centre: Vec2,
    heading: Vec2,
}

impl Contact {
    /// Whether the thing has neither moved nor sunk since `other`.
    fn same(&self, other: &Contact) -> bool {
        self.centre.distance_squared(other.centre) < 1e-4
            && self.heading.dot(other.heading) > 0.9998
            && (self.shape.depth - other.shape.depth).abs() < 0.02
            && (self.shape.radius - other.shape.radius).abs() < 0.01
            && self.shape.points.len() == other.shape.points.len()
    }
}

fn level(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// The eight corners of a box.
fn corners(centre: Vec3, turn: Quat, half: Vec3) -> [Vec3; 8] {
    std::array::from_fn(|i| {
        let sign = |bit: usize| if i & bit == 0 { -1.0 } else { 1.0 };
        centre + turn * (half * Vec3::new(sign(1), sign(2), sign(4)))
    })
}

/// The middle of a box's lowest corners (its bottom face when it lies
/// flat, one corner when it's tipped), and the height of its top.
fn low_and_top(corners: &[Vec3; 8]) -> (Vec3, f32) {
    let lowest = corners.iter().map(|c| c.y).fold(f32::INFINITY, f32::min);
    let top = corners
        .iter()
        .map(|c| c.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let low: Vec<Vec3> = corners
        .iter()
        .copied()
        .filter(|c| c.y <= lowest + 0.05)
        .collect();
    let middle = low.iter().sum::<Vec3>() / low.len() as f32;
    (Vec3::new(middle.x, lowest, middle.z), top)
}

/// How deep a thing whose lowest point is `sunk` studs into the snow
/// presses it, as a share of the snow's depth: `None` if it barely touches.
fn pressed(sunk: f32, plane: &Plane) -> Option<f32> {
    // A hundredth of a stud of slack, so a thing resting on the ground
    // presses the snow right through whatever the ray's rounding.
    let depth = ((sunk + 0.01) / plane.deep).min(1.0);
    (depth >= LIGHTEST).then_some(depth)
}

/// Where a box (its corners grown by `round` level) meets the snow: the
/// part of it under the snow's surface, seen from above.
fn box_contact(centre: Vec3, turn: Quat, half: Vec3, round: f32, plane: &Plane) -> Option<Contact> {
    let corners = corners(centre, turn, half);
    let over = corners.map(|c| plane.over(c));
    let lowest = over.iter().copied().fold(f32::INFINITY, f32::min);
    let depth = pressed(-lowest, plane)?;
    // The corners in the snow, and where the box's edges cross its surface.
    let mut points = Vec::with_capacity(12);
    for (i, &corner) in corners.iter().enumerate() {
        if over[i] <= 0.0 {
            points.push(level(corner));
        }
        for bit in [1, 2, 4] {
            let j = i | bit;
            if j != i && (over[i] <= 0.0) != (over[j] <= 0.0) {
                let t = over[i] / (over[i] - over[j]);
                points.push(level(corner.lerp(corners[j], t)));
            }
        }
    }
    let points = hull(points);
    if points.is_empty() {
        return None;
    }
    let centre = points.iter().sum::<Vec2>() / points.len() as f32;
    // No wider a wall than the outline is across.
    let across = round - outline_distance(&points, centre).min(0.0);
    let ahead = level(turn * Vec3::NEG_Z);
    let heading = if ahead.length_squared() > 0.01 {
        ahead.normalize()
    } else {
        level(turn * Vec3::Y).normalize_or(Vec2::X)
    };
    Some(Contact {
        shape: Shape {
            points,
            radius: round,
            soft: WALL.min(across * 0.9).max(0.04),
            depth,
        },
        centre,
        heading,
    })
}

/// Where a ball meets the snow: the circle its sunk part cuts in the
/// snow's surface, pressed into a bowl.
fn ball_contact(centre: Vec3, radius: f32, plane: &Plane) -> Option<Contact> {
    let over = plane.over(centre);
    let depth = pressed(radius - over, plane)?;
    let across = if over > 0.0 {
        (radius * radius - over * over).max(0.0).sqrt()
    } else {
        radius
    };
    if across < 0.02 {
        return None;
    }
    Some(Contact {
        shape: Shape {
            points: vec![level(centre)],
            radius: across,
            soft: across,
            depth,
        },
        centre: level(centre),
        heading: Vec2::X,
    })
}

/// A Part or MeshPart as it presses snow: its shape, place and size.
#[derive(Clone, Copy, Debug)]
struct Body {
    kind: PartType,
    centre: Vec3,
    turn: Quat,
    size: Vec3,
}

impl Body {
    /// `None` for anything that isn't a part with a place.
    fn of(instance: &Instance) -> Option<Body> {
        let vector = |name| {
            let v = instance.property(name).and_then(Value::as_vector3)?;
            let v = Vec3::new(v.x, v.y, v.z);
            v.is_finite().then_some(v)
        };
        // A MeshPart presses as its box: its model's own shape isn't read.
        let kind = if instance.class().has_model() {
            PartType::Block
        } else {
            instance.property("Shape").and_then(Value::as_part_type)?
        };
        Some(Body {
            kind,
            centre: vector("Position")?,
            turn: instance
                .property("Orientation")
                .and_then(Value::as_vector3)
                .map_or(Quat::IDENTITY, rotation_from_orientation),
            size: vector("Size")?.abs(),
        })
    }

    /// The box round it: a ball's and a cylinder's are as round as they are.
    fn half(&self) -> Vec3 {
        match self.kind {
            PartType::Ball => Vec3::splat(self.size.min_element() * 0.5),
            PartType::Cylinder | PartType::Capsule => {
                let across = self.size.y.min(self.size.z);
                Vec3::new(self.size.x, across, across) * 0.5
            }
            _ => self.size * 0.5,
        }
    }

    /// The point of it nearest the ground (the ground under it is found
    /// there) and the height of its top.
    fn low_and_top(&self) -> (Vec3, f32) {
        let half = self.half();
        match self.kind {
            PartType::Ball => (self.centre - Vec3::Y * half.x, self.centre.y + half.x),
            _ => low_and_top(&corners(self.centre, self.turn, half)),
        }
    }

    /// Where it meets the snow whose surface is `plane`.
    fn contact(&self, plane: &Plane) -> Option<Contact> {
        let half = self.half();
        let boxed = || box_contact(self.centre, self.turn, half, 0.0, plane);
        match self.kind {
            PartType::Ball => ball_contact(self.centre, half.x, plane),
            PartType::Cylinder | PartType::Capsule => {
                // It lies along its own X.
                let axis = self.turn * Vec3::X;
                let (reach, radius) = (half.x, half.y);
                let capsule = self.kind == PartType::Capsule;
                if axis.y.abs() < 0.25 {
                    // Lying: a strip along it, as wide as it has sunk, with
                    // a round floor.
                    let over = plane.over(self.centre);
                    let depth = pressed(radius - over, plane)?;
                    let across = if over > 0.0 {
                        (radius * radius - over * over).max(0.0).sqrt()
                    } else {
                        radius
                    };
                    if across < 0.02 {
                        return None;
                    }
                    let along = level(axis).normalize_or(Vec2::X);
                    let centre = level(self.centre);
                    let (points, round) = if capsule {
                        // Round ends: the line between its end balls.
                        let end = along * (reach - radius).max(0.0);
                        (hull(vec![centre - end, centre + end]), across)
                    } else {
                        let (end, side) = (along * reach, along.perp() * across);
                        (
                            hull(vec![
                                centre - end - side,
                                centre + end - side,
                                centre + end + side,
                                centre - end + side,
                            ]),
                            0.0,
                        )
                    };
                    Some(Contact {
                        shape: Shape {
                            points,
                            radius: round,
                            soft: across,
                            depth,
                        },
                        centre,
                        heading: along,
                    })
                } else if capsule {
                    // Standing or tipped: its lower end's ball.
                    let end = self.centre - axis * axis.y.signum() * (reach - radius).max(0.0);
                    ball_contact(end, radius, plane)
                } else if axis.y.abs() > 0.9 {
                    // Standing on its end: the end's disc, flat.
                    let end = self.centre - axis * axis.y.signum() * reach;
                    let depth = pressed(-plane.over(end), plane)?;
                    Some(Contact {
                        shape: Shape {
                            points: vec![level(end)],
                            radius,
                            soft: WALL.min(radius * 0.9).max(0.04),
                            depth,
                        },
                        centre: level(end),
                        heading: Vec2::X,
                    })
                } else {
                    // Tipped between the two: as the box round it.
                    boxed()
                }
            }
            // A wedge and a corner wedge press as the box round them too.
            _ => boxed(),
        }
    }
}

/// An outline kept so its print can be stamped again.
#[derive(Clone, Debug)]
struct Stamp {
    id: u64,
    shape: Shape,
    /// [`Prints::fill`] when it was stamped.
    born: f32,
    low: Vec2,
    high: Vec2,
}

/// A range of the map's cells: `x0..x1` by `z0..z1`.
#[derive(Clone, Copy, Debug)]
struct Cells {
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
}

const WHOLE: Cells = Cells {
    x0: 0,
    z0: 0,
    x1: SIDE,
    z1: SIDE,
};

/// The fine grid round the camera the prints are stamped into.
#[derive(Debug)]
struct Map {
    /// The world X and Z of cell (0, 0)'s corner, in studs.
    origin: [i32; 2],
    cells: Arc<WeatherPrintCells>,
    /// For each stud of it (8 by 8 cells), the most any of its cells holds:
    /// one that holds no more than everything has filled in is empty.
    blocks: Vec<u16>,
    /// And how deep its cells were pressed in all, when last added up.
    sums: Vec<Sum>,
    /// Whether any cell has been written since it was last blank.
    marked: bool,
    /// Whether cells changed this update ([`commit`](Self::commit)).
    changed: bool,
}

/// How deep a stud of the map's cells were pressed in all (`pressed`, in
/// levels) when everything had filled in by `at` (NaN: add it up again),
/// and how many of them were pressed at all. Until a cell fills in right
/// up, the sum goes down by a level for each of those as they fill in by
/// one, so it's added up again only every [`SUM_AGAIN`].
#[derive(Clone, Copy, Debug)]
struct Sum {
    at: f32,
    pressed: f32,
    cells: f32,
}

const STALE: Sum = Sum {
    at: f32::NAN,
    pressed: 0.0,
    cells: 0.0,
};
/// A stud's cells are added up again when they've filled in by this much.
const SUM_AGAIN: f32 = 0.03;

impl Map {
    fn new(origin: [i32; 2]) -> Self {
        Self {
            origin,
            cells: Arc::new(WeatherPrintCells {
                key: KEYS.fetch_add(1, Ordering::Relaxed),
                version: 1,
                origin: [origin[0] as f32, origin[1] as f32],
                cells_per_stud: CELLS,
                size: SIDE as u32,
                cells: vec![0.0; (SIDE * SIDE) as usize],
                row_versions: vec![1; SIDE as usize],
            }),
            blocks: vec![0; (STUDS * STUDS) as usize],
            sums: vec![STALE; (STUDS * STUDS) as usize],
            marked: false,
            changed: false,
        }
    }

    /// Where the map round a camera at `camera` starts.
    fn origin_for(camera: Vec3) -> [i32; 2] {
        let start = |at: f32| {
            let corner = at.floor() as i32 - STUDS / 2;
            (corner + SNAP / 2).div_euclid(SNAP) * SNAP
        };
        [start(camera.x), start(camera.z)]
    }

    /// The cells the box from `low` to `high` (world studs) touches, inside
    /// `within`.
    fn cells_in(&self, low: Vec2, high: Vec2, within: Cells) -> Cells {
        let cell = |at: f32, origin: i32| ((at - origin as f32) * CELLS).floor() as i32;
        Cells {
            x0: cell(low.x, self.origin[0]).max(within.x0),
            z0: cell(low.y, self.origin[1]).max(within.z0),
            x1: (cell(high.x, self.origin[0]) + 1).min(within.x1),
            z1: (cell(high.y, self.origin[1]) + 1).min(within.z1),
        }
    }

    /// Presses `shape` into the cells of `within` it covers, as it was
    /// stamped when everything had filled in by `born`; `fill` is that now.
    /// A cell keeps the deeper of what it holds and this.
    fn stamp(&mut self, shape: &Shape, born: f32, fill: f32, within: Cells) {
        let rim = shape.rim();
        if fill - born >= shape.depth.max(rim) {
            return;
        }
        let (low, high) = shape.bounds();
        let range = self.cells_in(low, high, within);
        if range.x0 >= range.x1 || range.z0 >= range.z1 {
            return;
        }
        let origin = Vec2::new(self.origin[0] as f32, self.origin[1] as f32);
        let sides = shape.sides();
        let (first, reach) = (
            shape.points.first().copied(),
            if rim > 0.0 { RIM_WIDTH } else { 0.0 },
        );
        // Copies the cells if a draw list still holds them.
        let map = Arc::make_mut(&mut self.cells);
        let next = map.version + 1;
        let level = |value: f32| {
            ((value + born) * WEATHER_PRINT_LEVELS)
                .round()
                .clamp(0.0, WEATHER_PRINT_MOST as f32) as u32
        };
        for z in range.z0..range.z1 {
            let mut row_changed = false;
            for x in range.x0..range.x1 {
                let p = origin + (Vec2::new(x as f32, z as f32) + 0.5) / CELLS;
                let away = sides_distance(&sides, first, p) - shape.radius;
                if away >= reach {
                    continue;
                }
                let index = (z * SIDE + x) as usize;
                let (mut press, mut lip) = WeatherPrintCells::unpack(map.cells[index]);
                let most = if away < 0.0 {
                    // Deepest in the middle: a wall that rounds into the floor.
                    let t = (-away / shape.soft).min(1.0);
                    let down = shape.depth * t * (2.0 - t);
                    let here = level(down);
                    if here <= press || down * WEATHER_PRINT_LEVELS < 0.5 {
                        continue;
                    }
                    press = here;
                    here
                } else {
                    // Highest a third of the way out, and down to nothing.
                    let u = away / RIM_WIDTH;
                    let raised = rim * 6.75 * u * (1.0 - u) * (1.0 - u);
                    let here = level(raised);
                    if here <= lip || raised * WEATHER_PRINT_LEVELS < 0.5 {
                        continue;
                    }
                    lip = here;
                    here
                };
                map.cells[index] = WeatherPrintCells::pack(press, lip);
                let block = ((z / CELLS as i32) * STUDS + x / CELLS as i32) as usize;
                self.blocks[block] = self.blocks[block].max(most as u16);
                self.sums[block].at = f32::NAN;
                row_changed = true;
            }
            if row_changed {
                map.row_versions[z as usize] = next;
                self.changed = true;
                self.marked = true;
            }
        }
    }

    /// Moves the map to start at `origin`: what it still covers keeps its
    /// cells, and the part it newly covers is stamped from `stamps`.
    fn scroll(&mut self, origin: [i32; 2], stamps: &VecDeque<Stamp>, fill: f32) {
        let dx = (origin[0] - self.origin[0]) * CELLS as i32;
        let dz = (origin[1] - self.origin[1]) * CELLS as i32;
        self.origin = origin;
        let map = Arc::make_mut(&mut self.cells);
        map.origin = [origin[0] as f32, origin[1] as f32];
        if !self.marked && stamps.is_empty() {
            // Blank, with nothing to bring back: the same cells do. (The
            // renderer reads the origin with them.)
            map.version += 1;
            return;
        }
        let side = SIDE as usize;
        let old = std::mem::replace(&mut map.cells, vec![0.0; side * side]);
        if dx.abs() < SIDE && dz.abs() < SIDE {
            // Cell (x, z) was (x + dx, z + dz).
            let (from, to) = (dx.max(0) as usize, (SIDE + dx.min(0)) as usize);
            let at = (-dx).max(0) as usize;
            for z in 0..SIDE {
                let was = z + dz;
                if !(0..SIDE).contains(&was) {
                    continue;
                }
                let (z, was) = (z as usize, was as usize);
                map.cells[z * side + at..z * side + at + (to - from)]
                    .copy_from_slice(&old[was * side + from..was * side + to]);
            }
        }
        let next = map.version + 1;
        map.row_versions.fill(next);
        self.changed = true;
        // What's newly covered: a strip of columns and a strip of rows (or
        // everything, after a jump).
        let jumped = dx.abs() >= SIDE || dz.abs() >= SIDE;
        let columns = Cells {
            x0: if dx > 0 { SIDE - dx } else { 0 },
            x1: if dx > 0 { SIDE } else { -dx },
            ..WHOLE
        };
        let rows = Cells {
            z0: if dz > 0 { SIDE - dz } else { 0 },
            z1: if dz > 0 { SIDE } else { -dz },
            ..WHOLE
        };
        for stamp in stamps {
            if jumped {
                self.stamp(&stamp.shape, stamp.born, fill, WHOLE);
                continue;
            }
            for strip in [columns, rows] {
                let covered = self.cells_in(stamp.low, stamp.high, strip);
                if covered.x0 < covered.x1 && covered.z0 < covered.z1 {
                    self.stamp(&stamp.shape, stamp.born, fill, strip);
                }
            }
        }
        self.recount();
    }

    /// Reads [`blocks`](Self::blocks) and [`marked`](Self::marked) again
    /// from the cells.
    fn recount(&mut self) {
        self.blocks.fill(0);
        self.sums.fill(STALE);
        let per = CELLS as usize;
        for (index, &cell) in self.cells.cells.iter().enumerate() {
            if cell == 0.0 {
                continue;
            }
            let (press, lip) = WeatherPrintCells::unpack(cell);
            let (x, z) = (index % SIDE as usize, index / SIDE as usize);
            let block = &mut self.blocks[(z / per) * STUDS as usize + x / per];
            *block = (*block).max(press.max(lip) as u16);
        }
        self.marked = self.blocks.iter().any(|&block| block > 0);
    }

    /// Counts every cell down by `levels`, as [`Prints::fill`] is.
    fn rebase(&mut self, levels: u32) {
        if !self.marked {
            return;
        }
        let map = Arc::make_mut(&mut self.cells);
        for cell in &mut map.cells {
            if *cell != 0.0 {
                let (press, lip) = WeatherPrintCells::unpack(*cell);
                *cell = WeatherPrintCells::pack(
                    press.saturating_sub(levels),
                    lip.saturating_sub(levels),
                );
            }
        }
        let next = map.version + 1;
        map.row_versions.fill(next);
        self.changed = true;
        self.recount();
    }

    /// Blanks it.
    fn clear(&mut self) {
        if !self.marked {
            return;
        }
        let map = Arc::make_mut(&mut self.cells);
        map.cells.fill(0.0);
        let next = map.version + 1;
        map.row_versions.fill(next);
        self.blocks.fill(0);
        self.marked = false;
        self.changed = true;
    }

    /// Ends an update: the cells' version goes up if they changed.
    fn commit(&mut self) {
        if std::mem::take(&mut self.changed) {
            Arc::make_mut(&mut self.cells).version += 1;
        }
    }

    /// How deep each stud of the map is pressed on average, with `fill`
    /// filled in, into channel 2 of the weather field's `cells` (`side`
    /// cells a side, a stud each, from `origin`), fading out toward the
    /// map's edge. That's what the terrain's corners are pressed down by,
    /// and what tells the shading that a print is near.
    fn average_into(&mut self, fill: f32, origin: [i32; 2], side: usize, cells: &mut [[f32; 4]]) {
        if !self.marked {
            return;
        }
        let filled = (fill * WEATHER_PRINT_LEVELS).max(0.0);
        let per = CELLS as usize;
        for bz in 0..STUDS {
            for bx in 0..STUDS {
                let block = &mut self.blocks[(bz * STUDS + bx) as usize];
                if *block == 0 {
                    continue;
                }
                if f32::from(*block) <= filled {
                    // Everything in it has filled in.
                    *block = 0;
                    continue;
                }
                let gx = self.origin[0] + bx - origin[0];
                let gz = self.origin[1] + bz - origin[1];
                if gx < 0 || gz < 0 || gx >= side as i32 || gz >= side as i32 {
                    continue;
                }
                let sum = &mut self.sums[(bz * STUDS + bx) as usize];
                if !(0.0..=SUM_AGAIN).contains(&(fill - sum.at)) {
                    *sum = Sum {
                        at: fill,
                        pressed: 0.0,
                        cells: 0.0,
                    };
                    for z in 0..per {
                        let row = (bz as usize * per + z) * SIDE as usize + bx as usize * per;
                        for &cell in &self.cells.cells[row..row + per] {
                            let (press, _) = WeatherPrintCells::unpack(cell);
                            let left = press as f32 - filled;
                            if left > 0.0 {
                                sum.pressed += left;
                                sum.cells += 1.0;
                            }
                        }
                    }
                }
                let pressed = sum.pressed - sum.cells * (fill - sum.at) * WEATHER_PRINT_LEVELS;
                let average = pressed.max(0.0) / ((per * per) as f32 * WEATHER_PRINT_LEVELS);
                let edge =
                    (bx.min(bz).min(STUDS - 1 - bx).min(STUDS - 1 - bz) as f32 + 0.5) / EDGE_FADE;
                // Never quite nothing where there's a print or a rim: the
                // shading reads the map only where this says one is near.
                cells[gz as usize * side + gx as usize][2] =
                    average.clamp(0.004, 1.0) * edge.min(1.0);
            }
        }
    }

    /// How far the snow is pressed down at a world point, with `fill`
    /// filled in.
    #[cfg(test)]
    fn pressed_at(&self, x: f32, z: f32, fill: f32) -> f32 {
        let cx = ((x - self.origin[0] as f32) * CELLS).floor() as i32;
        let cz = ((z - self.origin[1] as f32) * CELLS).floor() as i32;
        if cx < 0 || cz < 0 || cx >= SIDE || cz >= SIDE {
            return 0.0;
        }
        let (press, _) = WeatherPrintCells::unpack(self.cells.cells[(cz * SIDE + cx) as usize]);
        (press as f32 / WEATHER_PRINT_LEVELS - fill).max(0.0)
    }
}

/// A part of the Workspace that could press the snow.
#[derive(Clone, Copy, Debug)]
struct Tracked {
    id: InstanceId,
    /// Its `motion_revision` when it was last looked at.
    revision: u64,
    /// Whether it's been looked at while near the camera.
    looked: bool,
    /// The ground last found under it.
    ground: Option<Found>,
}

/// A thing that stands in the snow or walks over it, as the sky-visibility
/// rays should pass it by: it's no roof. Without this the ground under a
/// character or a crate counts as sheltered for as long as the field takes
/// to look again (bare patches and banked snow that trail whatever moves).
#[derive(Clone, Copy, Debug)]
pub(super) struct Passer {
    /// The Instance a ray passes by with what's inside it: a part, or a
    /// character's Model.
    pub skip: InstanceId,
    /// The box it's in.
    low: Vec3,
    high: Vec3,
}

impl Passer {
    fn of(skip: InstanceId, bodies: impl IntoIterator<Item = Body>) -> Option<Passer> {
        let mut bounds: Option<(Vec3, Vec3)> = None;
        for body in bodies {
            for corner in corners(body.centre, body.turn, body.half()) {
                let (low, high) = bounds.unwrap_or((corner, corner));
                bounds = Some((low.min(corner), high.max(corner)));
            }
        }
        bounds.map(|(low, high)| Passer { skip, low, high })
    }

    /// Whether a ray straight down at (`x`, `z`) that landed at height `y`
    /// may have landed on it (it has moved a little since its box was
    /// read, so half a stud round it counts).
    pub(super) fn stops(&self, x: f32, z: f32, y: f32) -> bool {
        x >= self.low.x - 0.5
            && x <= self.high.x + 0.5
            && z >= self.low.z - 0.5
            && z <= self.high.z + 0.5
            && y <= self.high.y + 0.1
    }
}

/// A thing that touches the snow: a part, or one foot of a character.
#[derive(Debug, Default)]
struct Mover {
    /// Where it met the snow last update, if it did.
    last: Option<Contact>,
    /// The kept outline its trail is still adding to.
    open: Option<Open>,
    /// [`Prints::fill`] when its print was last stamped.
    stamped: f32,
    /// The update it last moved in.
    moved: u64,
    /// A part's box, for the sky-visibility rays to pass by.
    passer: Option<Passer>,
}

/// The kept outline a trail is adding to.
#[derive(Clone, Copy, Debug)]
struct Open {
    stamp: u64,
    /// Where the thing was, and how it was turned, when the outline began.
    start: Vec2,
    heading: Vec2,
    /// The way the trail went first.
    way: Option<Vec2>,
    /// Whether the outline is the print under the thing as it lies still,
    /// which stays whole while it does ([`Prints::rest`]).
    resting: bool,
}

/// Which of a character's parts say where its feet are.
#[derive(Clone, Copy, Debug)]
enum Feet {
    /// A skinned body's foot sockets (Attachments whose `Bone` is its
    /// `LeftFoot` and `RightFoot`): where its feet are in the pose.
    Sockets {
        body: InstanceId,
        sockets: [InstanceId; 2],
    },
    /// A leg or foot part each side.
    Legs([InstanceId; 2]),
    /// Only the body's box.
    Root(InstanceId),
}

/// A character whose feet press the snow.
#[derive(Debug)]
struct Walker {
    feet: Feet,
    /// The Workspace's structure version `feet` was found at.
    structure: u64,
    /// The update it was last seen in.
    seen: u64,
    /// The lowest its foot bones have been over the ground: how far its
    /// soles are under them.
    ankle: f32,
    /// Where each foot bone was when it was last looked at, and the ground
    /// last found under it.
    bones: [Option<(Vec3, Quat)>; 2],
    grounds: [Option<Found>; 2],
    /// Where its body was last update, how far it has walked since its
    /// last step, and which foot steps next (a character whose feet aren't
    /// posed in the world).
    root: Option<Vec3>,
    walked: f32,
    left_next: bool,
    /// The box its parts are in, for the sky-visibility rays to pass by.
    passer: Option<Passer>,
}

/// What's pressed into the snow round the camera.
#[derive(Debug, Default)]
pub(super) struct Prints {
    map: Option<Map>,
    stamps: VecDeque<Stamp>,
    next_stamp: u64,
    /// How far every print has filled in, in whole depths, since the cells
    /// were last counted down.
    fill: f32,
    movers: HashMap<(InstanceId, u8), Mover>,
    /// The Workspace's parts that collide and aren't a character's.
    parts: Vec<Tracked>,
    parts_structure: Option<u64>,
    parts_age: f32,
    cursor: usize,
    walkers: HashMap<InstanceId, Walker>,
    updates: u64,
    /// Whether any print is filling in: one that isn't under a thing lying
    /// still in it.
    fading: bool,
}

impl Prints {
    /// Forgets every print, such as when the snow is gone.
    pub(super) fn clear(&mut self) {
        if self.map.is_some() || !self.parts.is_empty() || !self.walkers.is_empty() {
            *self = Self::default();
        }
    }

    /// The things in or on the snow, which the sky-visibility rays pass by:
    /// the parts that touch it and the characters.
    pub(super) fn passers(&self) -> Vec<Passer> {
        let parts = self.movers.values().filter_map(|mover| mover.passer);
        let characters = self.walkers.values().filter_map(|walker| walker.passer);
        parts.chain(characters).collect()
    }

    /// Whether any print is still filling in, so a host keeps drawing
    /// frames. The prints under things that lie still aren't.
    pub(super) fn is_fading(&self) -> bool {
        self.fading
    }

    /// The fine map for the shaders, while anything is pressed into it.
    pub(super) fn prints(&self) -> Option<WeatherPrints> {
        let map = self.map.as_ref().filter(|map| map.marked)?;
        Some(WeatherPrints {
            fill: self.fill,
            cells: map.cells.clone(),
        })
    }

    /// Writes how deep each cell of the weather field is pressed on average
    /// into its channel 2 ([`Map::average_into`]).
    pub(super) fn average_into(&mut self, origin: [i32; 2], side: usize, cells: &mut [[f32; 4]]) {
        if let Some(map) = &mut self.map {
            map.average_into(self.fill, origin, side, cells);
        }
    }

    /// Fills the prints in by `seconds`, moves the map to `camera`, and
    /// presses in what touches the snow now.
    pub(super) fn update(
        &mut self,
        world: &World,
        seconds: f32,
        camera: Vec3,
        snow: &Snow,
        surfaces: &mut Surfaces,
    ) {
        self.updates += 1;
        if self.fading {
            self.fill += seconds.max(0.0) * (1.0 + 1.5 * snow.falling) / snow.fade.max(0.5);
        }
        let fill = self.fill;
        self.stamps
            .retain(|stamp| fill - stamp.born < stamp.shape.life());
        if self.stamps.is_empty() {
            // All filled in: start again from blank, and what still lies in
            // the snow presses it again (`rest`).
            self.fill = 0.0;
            if let Some(map) = &mut self.map {
                map.clear();
            }
            for mover in self.movers.values_mut() {
                mover.stamped = f32::NEG_INFINITY;
            }
        } else if self.fill > REBASE {
            self.rebase();
        }
        let origin = Map::origin_for(camera);
        let map = self.map.get_or_insert_with(|| Map::new(origin));
        if map.origin != origin {
            map.scroll(origin, &self.stamps, self.fill);
        }

        let mut grounds = GROUNDS_AN_UPDATE;
        self.press_parts(world, seconds, camera, snow, surfaces, &mut grounds);
        self.press_feet(world, camera, snow, surfaces, &mut grounds);
        self.rest(world);
        if let Some(map) = &mut self.map {
            map.commit();
        }
        let held: HashSet<u64> = self
            .movers
            .values()
            .filter(|mover| mover.last.is_some())
            .filter_map(|mover| mover.open.filter(|open| open.resting))
            .map(|open| open.stamp)
            .collect();
        self.fading = self.stamps.iter().any(|stamp| !held.contains(&stamp.id));
    }

    /// Counts the cells and everything stamped down, so `fill` starts near
    /// 0 again.
    fn rebase(&mut self) {
        let levels = (self.fill * WEATHER_PRINT_LEVELS).floor();
        let by = levels / WEATHER_PRINT_LEVELS;
        self.fill -= by;
        for stamp in &mut self.stamps {
            stamp.born -= by;
        }
        for mover in self.movers.values_mut() {
            mover.stamped -= by;
        }
        if let Some(map) = &mut self.map {
            map.rebase(levels as u32);
        }
    }

    /// Keeps `shape` so its print can be stamped again, and returns its
    /// number.
    fn keep(&mut self, shape: Shape) -> u64 {
        if self.stamps.len() >= MOST_STAMPS {
            self.stamps.pop_front();
        }
        self.next_stamp += 1;
        let (low, high) = shape.bounds();
        self.stamps.push_back(Stamp {
            id: self.next_stamp,
            shape,
            born: self.fill,
            low,
            high,
        });
        self.next_stamp
    }

    /// Presses `shape` into the map now.
    fn stamp(&mut self, shape: &Shape) {
        if let Some(map) = &mut self.map {
            map.stamp(shape, self.fill, self.fill, WHOLE);
        }
    }

    /// A print of its own: stamped and kept.
    fn print(&mut self, contact: &Contact) {
        self.stamp(&contact.shape);
        self.keep(contact.shape.clone());
    }

    /// What `key` presses now: `contact`, or nothing. A thing that moved
    /// while touching the snow presses everything between where it was and
    /// where it is, so a thing dragged or rolled leaves a trail as wide as
    /// what touches.
    fn press(&mut self, key: (InstanceId, u8), contact: Option<Contact>) {
        let Some(contact) = contact else {
            self.movers.remove(&key);
            return;
        };
        let mover = self.movers.entry(key).or_default();
        let last = mover.last.take();
        let open = mover.open.take();
        if last.as_ref().is_some_and(|last| last.same(&contact)) {
            // It lies still: `rest` keeps its print fresh.
            mover.last = last;
            mover.open = open;
            return;
        }
        // Dragged from where it was, unless it was put here.
        let from = last.filter(|last| last.centre.distance(contact.centre) <= SWEEP_MOST);
        let swept = match &from {
            Some(from) => from.shape.joined(&contact.shape),
            None => contact.shape.clone(),
        };
        self.stamp(&swept);

        // Its trail's kept outline grows while the trail runs straight.
        let fill = self.fill;
        let grown = open.zip(from.as_ref()).and_then(|(open, from)| {
            let gone = contact.centre - open.start;
            let straight = open
                .way
                .is_none_or(|way| way.perp_dot(gone).abs() <= STAMP_STRAIGHT);
            if gone.length() > STAMP_LONGEST
                || !straight
                || contact.heading.dot(open.heading) < STAMP_TURN
            {
                return None;
            }
            let kept = self
                .stamps
                .iter_mut()
                .rev()
                .find(|stamp| stamp.id == open.stamp)?;
            kept.shape = kept.shape.joined(&contact.shape);
            kept.born = fill;
            (kept.low, kept.high) = kept.shape.bounds();
            let stepped = from.centre - open.start;
            Some(Open {
                way: open
                    .way
                    .or((stepped.length() > 0.05).then(|| stepped.normalize())),
                resting: false,
                ..open
            })
        });
        let open = match grown {
            Some(open) => open,
            None => Open {
                stamp: self.keep(swept),
                start: from.as_ref().map_or(contact.centre, |from| from.centre),
                heading: contact.heading,
                way: from
                    .as_ref()
                    .map(|from| contact.centre - from.centre)
                    .filter(|step| step.length() > 0.05)
                    .map(Vec2::normalize),
                resting: false,
            },
        };
        let mover = self.movers.entry(key).or_default();
        mover.last = Some(contact);
        mover.open = Some(open);
        mover.stamped = fill;
        mover.moved = self.updates;
    }

    /// The print under a thing that lies still stays whole: it's kept as
    /// an outline of its own, which isn't counted as filling in, and is
    /// stamped again before other prints' filling in has taken much of it.
    /// Things that are gone are forgotten.
    fn rest(&mut self, world: &World) {
        let (fill, updates) = (self.fill, self.updates);
        self.movers.retain(|&(id, _), _| world.contains(id));
        let still: Vec<_> = self
            .movers
            .iter()
            .filter(|(_, mover)| mover.moved != updates)
            .filter_map(|(&key, mover)| {
                let last = mover.last.as_ref()?;
                let stale = fill - mover.stamped > REST_AGAIN * last.shape.depth;
                let resting = mover.open.is_some_and(|open| open.resting);
                (stale || !resting).then(|| (key, last.clone(), mover.open, stale))
            })
            .collect();
        for (key, contact, open, stale) in still {
            // Its outline: the one it has, if that's this very print (not
            // the end of the trail it came by, which fills in).
            let have = open.and_then(|open| {
                self.stamps
                    .iter_mut()
                    .rev()
                    .find(|stamp| stamp.id == open.stamp)
                    .filter(|stamp| open.resting || stamp.shape == contact.shape)
            });
            let stamp = match have {
                Some(have) => {
                    if stale {
                        have.born = fill;
                    }
                    have.id
                }
                None => self.keep(contact.shape.clone()),
            };
            if stale {
                self.stamp(&contact.shape);
            }
            if let Some(mover) = self.movers.get_mut(&key) {
                mover.open = Some(Open {
                    stamp,
                    start: contact.centre,
                    heading: contact.heading,
                    way: None,
                    resting: true,
                });
                if stale {
                    mover.stamped = fill;
                }
            }
        }
    }

    /// Reads the Workspace's parts that could press snow again, if parts
    /// came or went (at most every [`PARTS_EVERY`] seconds): Parts and
    /// MeshParts that aren't a character's, kept in a tool or inside a
    /// UnionOperation.
    fn list_parts(&mut self, world: &World, seconds: f32) {
        self.parts_age += seconds.max(0.0);
        let structure = world.structure_version_of(world.workspace());
        if self.parts_structure == Some(structure)
            || (self.parts_structure.is_some() && self.parts_age < PARTS_EVERY)
        {
            return;
        }
        self.parts_structure = Some(structure);
        self.parts_age = 0.0;
        let had: HashMap<InstanceId, Tracked> =
            self.parts.drain(..).map(|part| (part.id, part)).collect();
        let characters = characters(world);
        let workspace = world.workspace();
        for class in [ClassName::Part, ClassName::MeshPart] {
            for id in world.all_of(class) {
                let Some(instance) = world.get(id) else {
                    continue;
                };
                // In the Workspace, and not in a character.
                let mut parent = instance.parent();
                let mut placed = false;
                while let Some(above) = parent {
                    if above == workspace {
                        placed = true;
                        break;
                    }
                    if characters.contains(&above) {
                        break;
                    }
                    parent = world.get(above).and_then(Instance::parent);
                }
                if !placed || axcenity_core::csg::out_of_world(world, id) {
                    continue;
                }
                self.parts.push(had.get(&id).copied().unwrap_or(Tracked {
                    id,
                    revision: instance.motion_revision(),
                    looked: false,
                    ground: None,
                }));
            }
        }
        self.cursor = 0;
    }

    /// Presses in the parts that moved, and the unanchored ones that lie in
    /// the snow near the camera.
    fn press_parts(
        &mut self,
        world: &World,
        seconds: f32,
        camera: Vec3,
        snow: &Snow,
        surfaces: &mut Surfaces,
        grounds: &mut usize,
    ) {
        self.list_parts(world, seconds);
        // The ones that moved, then a few of the rest in turn.
        let mut look: Vec<(usize, bool)> = Vec::new();
        for (index, part) in self.parts.iter().enumerate() {
            if world
                .get(part.id)
                .is_some_and(|instance| instance.motion_revision() != part.revision)
            {
                look.push((index, true));
            }
        }
        for _ in 0..LOOKS_AN_UPDATE.min(self.parts.len()) {
            let index = self.cursor % self.parts.len();
            self.cursor = index + 1;
            let part = self.parts[index];
            let Some(instance) = world.get(part.id) else {
                continue;
            };
            if instance.motion_revision() != part.revision {
                continue;
            }
            let near = Body::of(instance)
                .is_some_and(|body| level(body.centre).distance(level(camera)) <= RANGE);
            if near && !part.looked {
                look.push((index, false));
            } else if !near && part.looked {
                // The camera left it behind: its print fills in, and it
                // presses a new one when the camera is back.
                self.parts[index].looked = false;
                self.movers.remove(&(part.id, 0));
            }
        }

        for (index, moved) in look {
            let part = self.parts[index];
            let Some(instance) = world.get(part.id) else {
                continue;
            };
            let key = (part.id, 0);
            let near = Body::of(instance)
                .filter(|body| level(body.centre).distance(level(camera)) <= RANGE);
            let Some(body) = near else {
                self.parts[index].revision = instance.motion_revision();
                self.parts[index].looked = false;
                self.movers.remove(&key);
                continue;
            };
            let flag = |name| instance.property(name).and_then(Value::as_bool);
            // What nothing collides with presses nothing, and an anchored
            // part only when something moves it: a house stands in the snow
            // without a print under it.
            let presses =
                flag("CanCollide") != Some(false) && (moved || flag("Anchored") == Some(false));
            if !presses {
                self.parts[index].revision = instance.motion_revision();
                self.parts[index].looked = true;
                self.movers.remove(&key);
                continue;
            }
            let (low, top) = body.low_and_top();
            let ground = match part.ground.filter(|found| found.holds(low)) {
                Some(found) => found.ground,
                None => {
                    if *grounds == 0 {
                        // Next update: it still counts as moved.
                        continue;
                    }
                    *grounds -= 1;
                    let ground = ground_under(surfaces, part.id, &[(low, top, snow.depth())])[0];
                    self.parts[index].ground = Some(Found {
                        at: level(low),
                        low: low.y,
                        ground,
                    });
                    ground
                }
            };
            self.parts[index].revision = instance.motion_revision();
            self.parts[index].looked = true;
            let contact = ground
                .and_then(|ground| snow_over(snow, ground, top))
                .and_then(|plane| body.contact(&plane));
            self.press(key, contact);
            if let Some(mover) = self.movers.get_mut(&key) {
                mover.passer = Passer::of(part.id, [body]);
            }
        }
    }

    /// Presses in the feet of the characters near the camera.
    fn press_feet(
        &mut self,
        world: &World,
        camera: Vec3,
        snow: &Snow,
        surfaces: &mut Surfaces,
        grounds: &mut usize,
    ) {
        let structure = world.structure_version_of(world.workspace());
        let updates = self.updates;
        for model in characters(world) {
            let known = self
                .walkers
                .get(&model)
                .filter(|walker| walker.structure == structure)
                .map(|walker| walker.feet);
            let Some(feet) = known.or_else(|| find_feet(world, model)) else {
                continue;
            };
            let walker = self.walkers.entry(model).or_insert(Walker {
                feet,
                structure,
                seen: updates,
                ankle: f32::INFINITY,
                bones: [None; 2],
                grounds: [None; 2],
                root: None,
                walked: 0.0,
                left_next: false,
                passer: None,
            });
            walker.feet = feet;
            walker.structure = structure;
            walker.seen = updates;
            walker.passer = world.get(model).and_then(|instance| {
                let parts = instance.children().iter();
                Passer::of(
                    model,
                    parts.filter_map(|&id| world.get(id).and_then(Body::of)),
                )
            });
            match feet {
                Feet::Sockets { body, sockets } => {
                    self.press_posed_feet(
                        world, model, body, sockets, camera, snow, surfaces, grounds,
                    );
                }
                Feet::Legs(_) | Feet::Root(_) => {
                    self.press_steps(world, model, feet, camera, snow, surfaces, grounds);
                }
            }
        }
        // Characters that are gone.
        let movers = &mut self.movers;
        self.walkers.retain(|&model, walker| {
            let here = walker.seen == updates;
            if !here {
                movers.remove(&(model, 1));
                movers.remove(&(model, 2));
            }
            here
        });
    }

    /// A character whose skinned body has foot sockets: each foot presses
    /// as a foot-sized box, rounded, at its bone and turned as it is, so a
    /// foot prints where and when it's really down.
    #[allow(clippy::too_many_arguments)]
    fn press_posed_feet(
        &mut self,
        world: &World,
        model: InstanceId,
        body: InstanceId,
        sockets: [InstanceId; 2],
        camera: Vec3,
        snow: &Snow,
        surfaces: &mut Surfaces,
        grounds: &mut usize,
    ) {
        let Some(body) = world.get(body).and_then(Body::of) else {
            return;
        };
        // A socket's place is in its body's own space.
        let bone = |socket: InstanceId| {
            let socket = world.get(socket)?;
            let at = socket.property("Position").and_then(Value::as_vector3)?;
            let turn = socket
                .property("Orientation")
                .and_then(Value::as_vector3)
                .map_or(Quat::IDENTITY, rotation_from_orientation);
            let place = body.centre + body.turn * Vec3::new(at.x, at.y, at.z);
            place.is_finite().then_some((place, body.turn * turn))
        };
        let (Some(left), Some(right)) = (bone(sockets[0]), bone(sockets[1])) else {
            return;
        };
        let bones = [left, right];
        let Some(walker) = self.walkers.get_mut(&model) else {
            return;
        };
        let still = |was: Option<(Vec3, Quat)>, is: (Vec3, Quat)| {
            was.is_some_and(|was| {
                was.0.distance_squared(is.0) < 4e-6 && was.1.dot(is.1).abs() > 0.99999
            })
        };
        if still(walker.bones[0], left) && still(walker.bones[1], right) {
            return;
        }
        let middle = (left.0 + right.0) * 0.5;
        if level(middle).distance(level(camera)) > RANGE {
            walker.bones = [None; 2];
            self.movers.remove(&(model, 1));
            self.movers.remove(&(model, 2));
            return;
        }
        // The ground under each foot: found again for the ones that have
        // left where it was last found, both in one query.
        let lows = bones.map(|(place, _)| place - Vec3::Y * 0.01);
        let stale: Vec<usize> = (0..2)
            .filter(|&slot| !walker.grounds[slot].is_some_and(|found| found.holds(lows[slot])))
            .collect();
        if !stale.is_empty() && *grounds == 0 {
            return;
        }
        walker.bones = [Some(left), Some(right)];
        // Its height gives the size of its feet: a shod foot is about 15.5%
        // of a body's height long and 6.5% wide, the ankle about 4.5% up.
        let height = body.size.y;
        let top = middle.y + height;
        let reach = snow.depth() + 0.09 * height;
        if !stale.is_empty() {
            *grounds -= 1;
            let paths: Vec<(Vec3, f32, f32)> = stale
                .iter()
                .map(|&slot| (lows[slot], lows[slot].y + 0.5, reach))
                .collect();
            let found = ground_under(surfaces, model, &paths);
            for (&slot, ground) in stale.iter().zip(found) {
                walker.grounds[slot] = Some(Found {
                    at: level(lows[slot]),
                    low: lows[slot].y,
                    ground,
                });
            }
        }
        let found = walker
            .grounds
            .map(|found| found.and_then(|found| found.ground));
        for (slot, ((place, turn), ground)) in bones.into_iter().zip(found).enumerate() {
            let key = (model, slot as u8 + 1);
            let Some(ground) = ground else {
                self.press(key, None);
                continue;
            };
            // The lowest a foot bone has been over the ground is how far
            // the sole is under it.
            let Some(walker) = self.walkers.get_mut(&model) else {
                return;
            };
            // (The ground's plane, where the foot is now.)
            let under = ground.point.y
                - level(ground.normal).dot(level(place) - level(ground.point)) / ground.normal.y;
            walker.ankle = walker.ankle.min(place.y - under);
            let ankle = walker.ankle.clamp(0.02 * height, 0.09 * height);
            let (long, wide) = (0.155 * height, 0.065 * height);
            let round = 0.45 * wide;
            // The bone is over the heel end: about a quarter of the foot
            // is behind it.
            let centre = place + turn * Vec3::new(0.0, -ankle * 0.5, -0.23 * long);
            let half = Vec3::new(wide * 0.5 - round, ankle * 0.5, long * 0.5 - round);
            let contact = snow_over(snow, ground, top)
                .and_then(|plane| box_contact(centre, turn, half, round, &plane));
            self.press(key, contact);
        }
    }

    /// A character whose feet aren't posed in the world (its legs are
    /// parts that only follow its body, or it has only a body): a print
    /// each step, left and right in turn, at its real leg, turned as the
    /// leg is and as deep as the leg's foot is in the snow.
    #[allow(clippy::too_many_arguments)]
    fn press_steps(
        &mut self,
        world: &World,
        model: InstanceId,
        feet: Feet,
        camera: Vec3,
        snow: &Snow,
        surfaces: &mut Surfaces,
        grounds: &mut usize,
    ) {
        let body = |id: InstanceId| world.get(id).and_then(Body::of);
        let (root, stride) = match feet {
            Feet::Legs([left, _]) => {
                let Some(leg) = body(left) else {
                    return;
                };
                (leg, (0.62 * leg.size.y).clamp(0.5, 2.5))
            }
            Feet::Root(root) => {
                let Some(root) = body(root) else {
                    return;
                };
                (root, (0.25 * root.size.y).clamp(0.5, 2.5))
            }
            Feet::Sockets { .. } => return,
        };
        let Some(walker) = self.walkers.get_mut(&model) else {
            return;
        };
        let was = walker.root.replace(root.centre);
        if level(root.centre).distance(level(camera)) > RANGE {
            walker.root = None;
            return;
        }
        let step = was.map(|was| level(root.centre - was));
        // Both feet where it's first seen standing; then one a stride.
        let sides: &[bool] = match step {
            None => &[true, false],
            Some(step) if (1e-4..=20.0).contains(&step.length()) => {
                walker.walked += step.length();
                if walker.walked < stride {
                    return;
                }
                walker.walked = 0.0;
                walker.left_next = !walker.left_next;
                if walker.left_next { &[true] } else { &[false] }
            }
            Some(_) => return,
        };
        if *grounds == 0 {
            return;
        }
        *grounds -= 1;
        // Each foot as a low box under its leg.
        let boxes: Vec<(Vec3, Quat, Vec3, f32)> = sides
            .iter()
            .filter_map(|&left| match feet {
                Feet::Legs(legs) => {
                    let leg = body(legs[usize::from(!left)])?;
                    let wide = 0.85 * leg.size.x;
                    let long = (1.25 * leg.size.z).max(1.4 * wide);
                    let foot = leg.centre - leg.turn * Vec3::Y * (leg.size.y * 0.5);
                    Some((foot, leg.turn, Vec3::new(wide, 0.2, long), leg.centre.y))
                }
                _ => {
                    // The way it walks, or faces while it stands.
                    let ahead = step
                        .filter(|step| step.length() > 1e-4)
                        .map_or(level(root.turn * Vec3::NEG_Z), Vec2::normalize)
                        .normalize_or(Vec2::NEG_Y);
                    let turn = Quat::from_rotation_y((-ahead.x).atan2(-ahead.y));
                    let height = root.size.y;
                    let side = if left { -0.09 } else { 0.09 } * height;
                    let foot = root.centre - Vec3::Y * (height * 0.5) + turn * Vec3::X * side;
                    let size = Vec3::new(0.065 * height, 0.2, 0.155 * height);
                    Some((foot, turn, size, root.centre.y))
                }
            })
            .collect();
        let paths: Vec<(Vec3, f32, f32)> = boxes
            .iter()
            .map(|&(foot, _, _, top)| (foot, top.max(foot.y + 0.5), snow.depth()))
            .collect();
        let found = ground_under(surfaces, model, &paths);
        let top = root.centre.y + root.size.y;
        for ((foot, turn, size, _), ground) in boxes.into_iter().zip(found) {
            let round = 0.42 * size.x;
            let half = Vec3::new(size.x * 0.5 - round, size.y * 0.5, size.z * 0.5 - round);
            // Its toes a little ahead of the leg.
            let centre = foot + Vec3::Y * half.y + turn * Vec3::NEG_Z * (0.1 * size.z);
            let contact = ground
                .and_then(|ground| snow_over(snow, ground, top))
                .and_then(|plane| box_contact(centre, turn, half, round, &plane));
            if let Some(contact) = contact {
                self.print(&contact);
            }
        }
    }

    /// How far the snow is pressed down at a world point (0 for untouched).
    #[cfg(test)]
    pub(super) fn pressed_at(&self, x: f32, z: f32) -> f32 {
        self.map
            .as_ref()
            .map_or(0.0, |map| map.pressed_at(x, z, self.fill))
    }

    /// How many outlines are kept.
    #[cfg(test)]
    pub(super) fn kept(&self) -> usize {
        self.stamps.len()
    }
}

/// The Models in the Workspace with a Humanoid: the characters.
fn characters(world: &World) -> HashSet<InstanceId> {
    let workspace = world.workspace();
    world
        .all_of(ClassName::Humanoid)
        .filter_map(|humanoid| world.get(humanoid)?.parent())
        .filter(|&model| {
            world
                .get(model)
                .is_some_and(|model| model.class() == ClassName::Model)
        })
        .filter(|&model| {
            let mut parent = world.get(model).and_then(Instance::parent);
            while let Some(above) = parent {
                if above == workspace {
                    return true;
                }
                parent = world.get(above).and_then(Instance::parent);
            }
            false
        })
        .collect()
}

/// Which of a character's parts say where its feet are: a skinned body's
/// foot sockets if it has them, else a leg or foot part each side, else
/// its body's box.
fn find_feet(world: &World, model: InstanceId) -> Option<Feet> {
    let children = world.get(model)?.children();
    let named = |id: InstanceId| {
        let name = world.get(id)?.name();
        Some(
            name.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect::<String>(),
        )
    };
    for &child in children {
        let Some(part) = world.get(child).filter(|part| part.class().has_model()) else {
            continue;
        };
        let socket = |bone: &str| {
            part.children().iter().copied().find(|&id| {
                world.get(id).is_some_and(|socket| {
                    socket.class() == ClassName::Attachment
                        && socket
                            .attribute("Bone")
                            .and_then(Value::as_str)
                            .is_some_and(|name| name.eq_ignore_ascii_case(bone))
                })
            })
        };
        if let (Some(left), Some(right)) = (socket("LeftFoot"), socket("RightFoot")) {
            return Some(Feet::Sockets {
                body: child,
                sockets: [left, right],
            });
        }
    }
    let parts: Vec<(InstanceId, String)> = children
        .iter()
        .filter(|&&id| world.get(id).and_then(Body::of).is_some())
        .filter_map(|&id| Some((id, named(id)?)))
        .collect();
    let leg = |side: &str| {
        ["foot", "lowerleg", "leg"].iter().find_map(|limb| {
            let wanted = format!("{side}{limb}");
            parts
                .iter()
                .find(|(_, name)| *name == wanted)
                .map(|&(id, _)| id)
        })
    };
    if let (Some(left), Some(right)) = (leg("left"), leg("right")) {
        return Some(Feet::Legs([left, right]));
    }
    let root = parts
        .iter()
        .find(|(_, name)| name == "humanoidrootpart")
        .or(parts.first())?;
    Some(Feet::Root(root.0))
}

/// The ground under each of `points` (a point of a thing, the height to
/// start over it from, and how deep the snow can lie), passing `skip` and
/// what's inside it by: a ray down from the top of the thing to as far
/// under its lowest point as the snow could reach.
fn ground_under(
    surfaces: &mut Surfaces,
    skip: InstanceId,
    points: &[(Vec3, f32, f32)],
) -> Vec<Option<Ground>> {
    let paths: Vec<(Vector3, Vector3)> = points
        .iter()
        .map(|&(low, top, depth)| {
            // Twice the depth in a WeatherVolume, and a little for luck.
            let under = low.y - (2.0 * depth + THIN + 0.5);
            (
                Vector3::new(low.x, top.max(low.y) + 0.01, low.z),
                Vector3::new(low.x, under, low.z),
            )
        })
        .collect();
    surfaces(Some(skip), &paths)
        .into_iter()
        .map(|hit| {
            let hit = hit?;
            let point = Vec3::new(hit.position.x, hit.position.y, hit.position.z);
            let normal = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
            point.is_finite().then_some(Ground {
                point,
                normal: if normal.is_finite() && normal.y > 0.0 {
                    normal.normalize()
                } else {
                    Vec3::Y
                },
            })
        })
        .collect()
}
