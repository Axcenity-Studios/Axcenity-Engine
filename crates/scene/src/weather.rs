//! The world's weather (graphics Phase 27 + Section 4 Weather Systems):
//! Lighting's first `Weather`.
//!
//! One state holds it all: what falls (rain or snow, and how hard), how wet
//! surfaces are, how much snow lies on them and how thick it is, sky-visibility
//! occlusion against roofs/solid geometry, manual interior/exterior paint
//! overrides, and dynamic snow footprints/trails.
//!
//! [`settled`] is the target surface state as the world says it should be,
//! which a draw list built from the world alone shows. A host's [`WeatherState`]
//! (inside its `Particles`) eases toward it over the Weather's `TransitionTime`,
//! occludes falling drops against solid roofs and interior volumes, presses
//! what touches the snow into it ([`prints`]: footprints, ruts and the
//! hollows things lie in), and produces the [`WeatherField`] uploaded to the
//! GPU for exterior surface masking and those prints.
//!
//! A `WeatherVolume` (an invisible part in the Workspace) sets the weather
//! in its box by hand, in place of that shelter: [`weather_volumes`] reads
//! them, the draw list carries their boxes ([`weather_boxes`]) for the
//! shaders, and a host's [`WeatherState`] keeps rain and snow out of the
//! ones whose `Precipitation` is off and lets them into the others.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

use axcenity_core::{ClassName, Instance, Value, Vector3, World};
use axcenity_render::glam::{Mat4, Quat, Vec3};
use axcenity_render::{
    Camera, DEFAULT_SNOW_THICKNESS, FlipbookFrame, Sprite, TextureSource, WEATHER_FIELD_SIZE,
    Weather, WeatherBox, WeatherField, WeatherPrints,
};

use crate::orientation::rotation_from_orientation;
use crate::particles::{
    FLAKE_GRID, Random, Surfaces, global_wind, rain_splash, rain_streak, snow_flakes,
};

mod prints;
#[cfg(test)]
mod prints_tests;

/// The most raindrops and snowflakes falling at once, at Intensity 1.
const MOST_DROPS: usize = 7000;
const MOST_FLAKES: usize = 8000;
/// The box around the camera they fall in, in studs. A drop leaving one
/// side comes back in the other, so the camera never runs out of them.
const BOX: Vec3 = Vec3::new(64.0, 48.0, 64.0);
/// How fast rain and snow fall in still air, in studs a second.
const RAIN_SPEED: f32 = 60.0;
const SNOW_SPEED: f32 = 5.0;
/// How far snowflakes sway from side to side as they fall, in studs a
/// second.
const SNOW_SWAY: f32 = 1.5;
/// A drop this near the camera (in studs) isn't drawn, and it fades in out
/// to the second distance: one a hand's width from the lens would cross
/// the whole picture as a bar (rain) or a disc (snow).
const RAIN_NEAR: (f32, f32) = (2.0, 10.0);
const SNOW_NEAR: (f32, f32) = (2.5, 9.0);
/// The most a thing counts as standing over the ground for the snow that
/// drifts against it (studs), and how many cells round a cell are searched
/// for the ground it stands on ([`bank_heights`]).
const BANK_MOST: f32 = 3.0;
const BANK_REACH: i32 = 3;
/// Spindrift: lying snow the wind picks up and runs along the ground. The
/// most streaks of it at once; the wind (level, studs a second) from which
/// it starts and by which there's all of it; and how high over the snow
/// the highest run, in studs.
const MOST_SPINDRIFT: usize = 2600;
const SPINDRIFT_WIND: (f32, f32) = (2.0, 13.0);
const SPINDRIFT_HEIGHT: f32 = 0.55;
/// A snowflake's picture is this many studs across (the clump in it about
/// a third of that), times 0.6 to 1.8: most are small, a few are clumps.
const FLAKE_SIZE: f32 = 0.2;
/// Nearer the camera than this many studs a flake is out of focus: up to
/// this many times as wide right at the lens, and as much fainter as it
/// is wider, so the nearest drift past as soft blurs and not as discs.
const FLAKE_BLUR: (f32, f32) = (14.0, 2.6);

/// How far the scene's light shows on rain and snow (`Sprite::light_influence`):
/// they dim with the day and under cloud, and keep a quarter of their own
/// color, so falling snow still shows against a night sky.
const WEATHER_LIGHT: f32 = 0.75;

/// Side length of the sky-visibility / surface-mask / footprint grid around the
/// camera, in cells.
const FIELD_SIZE: usize = WEATHER_FIELD_SIZE as usize;
/// World studs per weather field cell.
const FIELD_CELL_SIZE: f32 = 1.0;
/// Sentinel `roof_y` meaning open sky (no solid blocker above).
const OPEN_SKY_Y: f32 = -1.0e9;
/// Maximum top-down sky-visibility rays cast per frame.
const ROOF_RAYS_PER_FRAME: usize = 512;
/// How high above the camera sky-visibility rays start, in studs.
const SKY_RAY_HEIGHT: f32 = 128.0;
/// How far below the camera sky-visibility rays reach, in studs.
const SKY_RAY_DEPTH: f32 = 64.0;
/// How many raindrops a second splash on what's under them at Intensity 1,
/// within [`SPLASH_REACH`] studs in front of the camera, and the most that
/// land in one update (each is a ray against the world's solid surfaces).
const SPLASHES_A_SECOND: f32 = 900.0;
const SPLASHES_AN_UPDATE: usize = 64;
const SPLASH_REACH: f32 = 34.0;
/// How long a splash lasts, in seconds, and how wide its crown of spray
/// gets, in studs (each ±30%).
const SPLASH_LIFE: f32 = 0.16;
const SPLASH_SIZE: f32 = 0.24;
/// Rain runs off the edges of roofs and ledges: where the ground under the
/// sky drops by more than this many studs from one field cell to the next,
/// drips fall from the higher one's edge.
const DRIP_LEDGE: f32 = 1.5;
/// How many drips a second fall from each stud of such an edge at Intensity
/// 1, the most that start in one update, and the most falling at once.
const DRIPS_A_SECOND: f32 = 7.0;
const DRIPS_AN_UPDATE: usize = 48;
const MOST_DRIPS: usize = 900;
/// How fast a drip gains speed, in studs a second each second (gravity on
/// a 0.35 m stud), and how long its streak is at most, in studs.
const DRIP_GRAVITY: f32 = 28.0;
const DRIP_LENGTH: f32 = 0.7;
/// The most things in the snow whose sky-visibility rays are cast again
/// past them in one update (a query each).
const PASSERS_AN_UPDATE: usize = 4;
/// How far over the ground (studs) what's above it starts to shelter it, and
/// from where it shelters it wholly: lighting.wgsl's `SHELTER_LOW`.
const SHELTER_LOW: (f32, f32) = (2.5, 4.5);

/// Manual interior/exterior surface override painted on a 1-stud world XZ cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceMaskKind {
    /// Forces the cell to stay dry and snow-free and blocks precipitation.
    Interior,
    /// Forces the cell to be exposed to rain/snow even under geometry.
    Exterior,
}

impl SurfaceMaskKind {
    fn as_shader_value(self) -> f32 {
        match self {
            Self::Interior => -1.0,
            Self::Exterior => 1.0,
        }
    }
}

/// Parses a `Weather.SurfaceMask` property string (`"x,z:I;x,z:E"`) into a
/// sorted map of integer XZ cells to manual overrides.
pub fn parse_surface_mask(raw: &str) -> BTreeMap<(i32, i32), SurfaceMaskKind> {
    let mut map = BTreeMap::new();
    for entry in raw.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some((coords, kind)) = entry.split_once(':') else {
            continue;
        };
        let Some((xs, zs)) = coords.split_once(',') else {
            continue;
        };
        let (Ok(x), Ok(z)) = (xs.trim().parse::<i32>(), zs.trim().parse::<i32>()) else {
            continue;
        };
        let kind = match kind.trim() {
            "I" | "i" | "interior" => SurfaceMaskKind::Interior,
            "E" | "e" | "exterior" => SurfaceMaskKind::Exterior,
            _ => continue,
        };
        map.insert((x, z), kind);
    }
    map
}

/// Serializes a manual interior/exterior cell map back into `Weather.SurfaceMask`.
pub fn format_surface_mask(mask: &BTreeMap<(i32, i32), SurfaceMaskKind>) -> String {
    let mut out = String::new();
    for (&(x, z), &kind) in mask {
        if !out.is_empty() {
            out.push(';');
        }
        let code = match kind {
            SurfaceMaskKind::Interior => 'I',
            SurfaceMaskKind::Exterior => 'E',
        };
        out.push_str(&format!("{x},{z}:{code}"));
    }
    out
}

/// Paints (or erases when `kind` is `None`) all integer XZ cells within
/// `radius` studs of `(world_x, world_z)`. Returns `true` if the mask changed.
pub fn paint_surface_mask(
    mask: &mut BTreeMap<(i32, i32), SurfaceMaskKind>,
    world_x: f32,
    world_z: f32,
    radius: f32,
    kind: Option<SurfaceMaskKind>,
) -> bool {
    let radius = radius.max(0.5);
    let r2 = radius * radius;
    let min_x = (world_x - radius).floor() as i32;
    let max_x = (world_x + radius).floor() as i32;
    let min_z = (world_z - radius).floor() as i32;
    let max_z = (world_z + radius).floor() as i32;
    let mut changed = false;
    for cz in min_z..=max_z {
        for cx in min_x..=max_x {
            let dx = (cx as f32 + 0.5) - world_x;
            let dz = (cz as f32 + 0.5) - world_z;
            if dx * dx + dz * dz > r2 {
                continue;
            }
            match kind {
                Some(k) => {
                    if mask.insert((cx, cz), k) != Some(k) {
                        changed = true;
                    }
                }
                None => {
                    if mask.remove(&(cx, cz)).is_some() {
                        changed = true;
                    }
                }
            }
        }
    }
    changed
}

/// What the world's Weather asks for, each from 0 to 1 (except thickness and
/// fade/transition times).
#[derive(Clone, Debug, PartialEq)]
struct Target {
    /// How hard rain and snow fall (only one of them is above 0).
    rain: f32,
    snow: f32,
    /// How wet surfaces are, and how much snow covers them: at least as
    /// much as what's falling.
    wetness: f32,
    cover: f32,
    /// Snow layer thickness in studs (0..2).
    snow_thickness: f32,
    /// How often lightning flashes (0 to 1).
    lightning: f32,
    /// Seconds a change takes to ease in.
    transition: f32,
    /// Whether sky-visibility occlusion is active.
    occlusion: bool,
    /// Whether dynamic snow footprints are active.
    footprints: bool,
    /// Seconds footprints take to fade back out.
    footprint_fade: f32,
    /// Raw `SurfaceMask` string for change detection.
    surface_mask: String,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            rain: 0.0,
            snow: 0.0,
            wetness: 0.0,
            cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS,
            lightning: 0.0,
            transition: 0.0,
            occlusion: true,
            footprints: true,
            footprint_fade: 30.0,
            surface_mask: String::new(),
        }
    }
}

impl Target {
    fn of(world: &World) -> Self {
        world
            .get(world.lighting())
            .and_then(|lighting| weather_in(world, lighting))
            .map_or_else(Self::default, Self::read)
    }

    fn read(weather: &Instance) -> Self {
        let share = |name| match weather.property(name) {
            Some(Value::Number(value)) if value.is_finite() => value.clamp(0.0, 1.0) as f32,
            _ => 0.0,
        };
        let enabled = weather
            .property("Enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let transition = match weather.property("TransitionTime") {
            Some(Value::Number(value)) if value.is_finite() => value.max(0.0) as f32,
            _ => 0.0,
        };
        let snow_thickness = match weather.property("SnowThickness") {
            Some(Value::Number(value)) if value.is_finite() => value.clamp(0.0, 2.0) as f32,
            _ => DEFAULT_SNOW_THICKNESS,
        };
        let occlusion = weather
            .property("OcclusionEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let footprints = weather
            .property("FootprintsEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let footprint_fade = match weather.property("FootprintFadeTime") {
            Some(Value::Number(value)) if value.is_finite() => value.max(0.5) as f32,
            _ => 30.0,
        };
        let surface_mask = match weather.property("SurfaceMask") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        if !enabled {
            return Self {
                rain: 0.0,
                snow: 0.0,
                wetness: 0.0,
                cover: 0.0,
                snow_thickness,
                lightning: 0.0,
                transition,
                occlusion,
                footprints,
                footprint_fade,
                surface_mask,
            };
        }
        let intensity = share("Intensity");
        let falling = match weather.property("Precipitation") {
            Some(Value::EnumItem(item)) => item.item.as_str(),
            _ => "None",
        };
        let rain = if falling == "Rain" { intensity } else { 0.0 };
        let snow = if falling == "Snow" { intensity } else { 0.0 };
        Self {
            rain,
            snow,
            wetness: share("Wetness").max(rain),
            cover: share("SnowCover").max(snow),
            snow_thickness,
            lightning: share("Lightning"),
            transition,
            occlusion,
            footprints,
            footprint_fade,
            surface_mask,
        }
    }

    fn look(&self) -> Weather {
        Weather {
            wetness: self.wetness,
            snow: self.cover,
            snow_thickness: if self.cover > 0.0 {
                self.snow_thickness
            } else {
                0.0
            },
            rain: self.rain,
            snowfall: self.snow,
            lightning: self.lightning,
        }
    }
}

/// Lighting's first Weather, if it has one.
fn weather_in<'a>(world: &'a World, lighting: &Instance) -> Option<&'a Instance> {
    lighting
        .children()
        .iter()
        .filter_map(|&id| world.get(id))
        .find(|child| child.class() == ClassName::Weather)
}

/// How wet and snowy the world's surfaces are once its Weather has eased
/// in. Without a Weather, dry and bare.
pub(crate) fn settled(world: &World) -> Weather {
    Target::of(world).look()
}

/// The weather now, partway to what the world asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Eased {
    rain: f32,
    snow: f32,
    wetness: f32,
    cover: f32,
    snow_thickness: f32,
    lightning: f32,
}

impl Eased {
    fn settled(target: &Target) -> Self {
        Self {
            rain: target.rain,
            snow: target.snow,
            wetness: target.wetness,
            cover: target.cover,
            snow_thickness: target.snow_thickness,
            lightning: target.lightning,
        }
    }
}

/// One raindrop or snowflake.
#[derive(Clone, Copy, Debug)]
struct Drop {
    /// Where it is inside [`BOX`], repeated across the world.
    position: Vec3,
    /// How much faster or slower than the rest it falls.
    speed: f32,
    /// Where in its sway a snowflake is.
    phase: f32,
}

/// A raindrop's splash: a crown of spray thrown up from where it landed.
#[derive(Clone, Copy, Debug)]
struct Splash {
    /// Where the drop landed.
    position: Vec3,
    /// Seconds since it landed, and how long it lasts.
    age: f32,
    life: f32,
    /// How wide its crown gets, in studs.
    size: f32,
}

/// A drip of rainwater falling from the edge of a roof or ledge.
#[derive(Clone, Copy, Debug)]
struct Drip {
    position: Vec3,
    /// How fast it falls now, in studs a second.
    speed: f32,
    /// The height of what it lands on.
    floor: f32,
}

/// One interior `GraphicsVolume` box (a `GraphicsVolume` with a `FogEffect`
/// whose `Interior` is true), cached for precipitation occlusion.
#[derive(Clone, Copy, Debug)]
struct InteriorBox {
    center: Vec3,
    inv_rotation: Quat,
    half_extent: Vec3,
}

impl InteriorBox {
    fn contains(&self, point: Vec3) -> bool {
        let local = self.inv_rotation * (point - self.center);
        local.x.abs() <= self.half_extent.x
            && local.y.abs() <= self.half_extent.y
            && local.z.abs() <= self.half_extent.z
    }
}

/// One enabled `WeatherVolume`: the box the shaders take, and whether rain
/// and snow fall inside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WeatherVolume {
    pub weather_box: WeatherBox,
    /// The volume's `Precipitation`: false keeps drops out even under open
    /// sky, true lets them fall even under a roof.
    pub precipitation: bool,
}

impl WeatherVolume {
    /// Whether `point` is in the box (at its faces: drops don't fade).
    fn contains(&self, point: Vec3) -> bool {
        let local = self.weather_box.to_box.transform_point3(point);
        local.abs().cmple(self.weather_box.half_size).all()
    }
}

/// The world's enabled WeatherVolumes in the Workspace (not the ones kept
/// inside a UnionOperation), in the order they apply: lowest `Priority`
/// first, equal ones in Explorer order, so of two that overlap the later
/// one wins.
pub(crate) fn weather_volumes(world: &World) -> Vec<WeatherVolume> {
    let mut found: Vec<(f64, WeatherVolume)> = world
        .all_below(world.workspace(), |class| class == ClassName::WeatherVolume)
        .into_iter()
        .filter(|&id| !axcenity_core::csg::out_of_world(world, id))
        .filter_map(|id| world.get(id))
        .filter(|volume| volume.property("Enabled") != Some(&Value::Bool(false)))
        .filter_map(|volume| {
            let number = |name: &str| volume.property(name).and_then(Value::as_number);
            // A share of the world's wetness or snow, from 0 to 2 (a NaN
            // counts as none).
            let share = |name: &str| match number(name) {
                Some(n) if n > 0.0 => n.min(2.0) as f32,
                _ => 0.0,
            };
            // Its box is its Size, whatever its Shape.
            let vector = |name: &str| {
                let v = volume.property(name).and_then(Value::as_vector3)?;
                Some(Vec3::new(v.x, v.y, v.z))
            };
            let (center, size) = (vector("Position")?, vector("Size")?);
            let rotation = volume
                .property("Orientation")
                .and_then(Value::as_vector3)
                .map_or(Quat::IDENTITY, rotation_from_orientation);
            let priority = number("Priority").filter(|n| !n.is_nan()).unwrap_or(0.0);
            let volume = WeatherVolume {
                weather_box: WeatherBox {
                    to_box: Mat4::from_rotation_translation(rotation, center).inverse(),
                    half_size: size.abs() * 0.5,
                    blend: number("BlendDistance").map_or(0.0, |n| n.max(0.0)) as f32,
                    wetness: share("Wetness"),
                    snow: share("Snow"),
                },
                precipitation: volume.property("Precipitation") == Some(&Value::Bool(true)),
            };
            Some((priority, volume))
        })
        .collect();
    // A stable sort, so equal priorities keep Explorer order.
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, volume)| volume).collect()
}

/// [`weather_volumes`]' boxes, for the draw list.
pub(crate) fn weather_boxes(world: &World) -> Vec<WeatherBox> {
    weather_volumes(world)
        .into_iter()
        .map(|volume| volume.weather_box)
        .collect()
}

/// The share of the world's snow that lies on the ground at `ground`, under
/// a thing whose top is at `top`: what a WeatherVolume there sets (the last
/// one the point is in, at its faces: its `BlendDistance` isn't counted),
/// else none in a cell painted Interior or in an Interior GraphicsVolume,
/// else none under a roof. The roof is the sky-visibility grid's height in
/// the point's cell, which may be the thing's own top: only what stands
/// over the thing shelters the ground under it, and only from
/// [`SHELTER_LOW`] over the ground, as in lighting.wgsl's
/// `weather_cell_exposure` (which blends between cells; this reads one).
fn snow_share(
    volumes: &[WeatherVolume],
    manual_mask: &BTreeMap<(i32, i32), SurfaceMaskKind>,
    interiors: &[InteriorBox],
    grid: Option<&SkyGrid>,
    ground: Vec3,
    top: f32,
) -> f32 {
    if let Some(volume) = volumes.iter().rev().find(|volume| volume.contains(ground)) {
        return volume.weather_box.snow;
    }
    match manual_mask.get(&(ground.x.floor() as i32, ground.z.floor() as i32)) {
        Some(SurfaceMaskKind::Interior) => return 0.0,
        Some(SurfaceMaskKind::Exterior) => return 1.0,
        None => {}
    }
    if interiors.iter().any(|interior| interior.contains(ground)) {
        return 0.0;
    }
    match grid.and_then(|grid| grid.roof_at(ground.x, ground.z)) {
        Some(roof) if roof > OPEN_SKY_Y * 0.5 && roof > top + 0.3 => {
            let t = ((roof - ground.y - SHELTER_LOW.0) / (SHELTER_LOW.1 - SHELTER_LOW.0))
                .clamp(0.0, 1.0);
            1.0 - t * t * (3.0 - 2.0 * t)
        }
        _ => 1.0,
    }
}

/// Cached 64×64 world-space sky-visibility ceiling height grid around the camera.
#[derive(Debug)]
struct SkyGrid {
    origin: [i32; 2],
    /// `FIELD_SIZE * FIELD_SIZE` highest solid surface Y seen from above, or
    /// [`OPEN_SKY_Y`] if open sky.
    roof_y: Vec<f32>,
    /// True if cell has been raycast at least once at its current world XZ.
    sampled: Vec<bool>,
    /// Rolling refresh index so moving roofs/doors eventually update.
    cursor: usize,
    /// Last camera Y used for ray height bounds.
    camera_y: f32,
}

impl SkyGrid {
    fn new(origin: [i32; 2], camera_y: f32) -> Self {
        let count = FIELD_SIZE * FIELD_SIZE;
        Self {
            origin,
            roof_y: vec![OPEN_SKY_Y; count],
            sampled: vec![false; count],
            cursor: 0,
            camera_y,
        }
    }

    /// Shifts the grid to `new_origin`, preserving any cells that still overlap.
    fn recenter(&mut self, new_origin: [i32; 2], camera_y: f32) {
        self.camera_y = camera_y;
        if new_origin == self.origin {
            return;
        }
        let count = FIELD_SIZE * FIELD_SIZE;
        let mut next_roof = vec![OPEN_SKY_Y; count];
        let mut next_sampled = vec![false; count];
        let dx = new_origin[0] - self.origin[0];
        let dz = new_origin[1] - self.origin[1];
        for z in 0..FIELD_SIZE {
            let old_z = z as i32 + dz;
            if old_z < 0 || old_z >= FIELD_SIZE as i32 {
                continue;
            }
            for x in 0..FIELD_SIZE {
                let old_x = x as i32 + dx;
                if old_x < 0 || old_x >= FIELD_SIZE as i32 {
                    continue;
                }
                let old_idx = (old_z as usize) * FIELD_SIZE + (old_x as usize);
                let new_idx = z * FIELD_SIZE + x;
                next_roof[new_idx] = self.roof_y[old_idx];
                next_sampled[new_idx] = self.sampled[old_idx];
            }
        }
        self.origin = new_origin;
        self.roof_y = next_roof;
        self.sampled = next_sampled;
    }

    /// Casts up to [`ROOF_RAYS_PER_FRAME`] top-down rays via `surfaces`,
    /// prioritizing unsampled cells first and then rolling through existing
    /// cells so dynamic roofs/doors stay fresh. A ray that lands on one of
    /// `passers` (a character, or a part lying in the snow) is cast again
    /// past it: what moves over the ground is no roof to it.
    fn update(&mut self, surfaces: &mut Surfaces, passers: &[prints::Passer]) {
        let count = FIELD_SIZE * FIELD_SIZE;
        let mut indices = Vec::with_capacity(ROOF_RAYS_PER_FRAME);
        for (idx, &done) in self.sampled.iter().enumerate() {
            if !done {
                indices.push(idx);
                if indices.len() >= ROOF_RAYS_PER_FRAME {
                    break;
                }
            }
        }
        if indices.len() < ROOF_RAYS_PER_FRAME {
            let extra = (ROOF_RAYS_PER_FRAME - indices.len()).min(128);
            for _ in 0..extra {
                let idx = self.cursor % count;
                self.cursor = (self.cursor + 1) % count;
                if self.sampled[idx] {
                    indices.push(idx);
                }
            }
        }
        if indices.is_empty() {
            return;
        }
        let top_y = self.camera_y + SKY_RAY_HEIGHT;
        let bottom_y = self.camera_y - SKY_RAY_DEPTH;
        let paths: Vec<(Vector3, Vector3)> = indices
            .iter()
            .map(|&idx| {
                let gx = (idx % FIELD_SIZE) as i32;
                let gz = (idx / FIELD_SIZE) as i32;
                let wx = (self.origin[0] + gx) as f32 + 0.5;
                let wz = (self.origin[1] + gz) as f32 + 0.5;
                (Vector3::new(wx, top_y, wz), Vector3::new(wx, bottom_y, wz))
            })
            .collect();
        let mut hits = surfaces(None, &paths);
        if !passers.is_empty() {
            // Each passer's rays in one query, for the first few of them.
            let mut again: Vec<(usize, Vec<usize>)> = Vec::new();
            for (ray, hit) in hits.iter().enumerate() {
                let Some(hit) = hit else {
                    continue;
                };
                let (x, z) = (paths[ray].0.x, paths[ray].0.z);
                let Some(passer) = passers
                    .iter()
                    .position(|passer| passer.stops(x, z, hit.position.y))
                else {
                    continue;
                };
                match again.iter_mut().find(|(known, _)| *known == passer) {
                    Some((_, rays)) => rays.push(ray),
                    None => again.push((passer, vec![ray])),
                }
            }
            for (passer, rays) in again.into_iter().take(PASSERS_AN_UPDATE) {
                let past: Vec<(Vector3, Vector3)> = rays.iter().map(|&ray| paths[ray]).collect();
                let found = surfaces(Some(passers[passer].skip), &past);
                for (ray, hit) in rays.into_iter().zip(found) {
                    hits[ray] = hit;
                }
            }
        }
        for (&idx, hit) in indices.iter().zip(hits) {
            self.roof_y[idx] = match hit {
                Some(hit) => hit.position.y,
                None => OPEN_SKY_Y,
            };
            self.sampled[idx] = true;
        }
    }

    /// Looks up the cached roof height at world `(x, z)`, if inside the grid.
    fn roof_at(&self, x: f32, z: f32) -> Option<f32> {
        let gx = (x.floor() as i32) - self.origin[0];
        let gz = (z.floor() as i32) - self.origin[1];
        if gx < 0 || gz < 0 || gx >= FIELD_SIZE as i32 || gz >= FIELD_SIZE as i32 {
            return None;
        }
        Some(self.roof_y[(gz as usize) * FIELD_SIZE + (gx as usize)])
    }
}

/// A host's weather: eased toward the world's, with its rain, snow,
/// sky-visibility occlusion grid, manual surface masks, and snow footprints.
#[derive(Debug, Default)]
pub(crate) struct WeatherState {
    /// `None` until the first update, which takes the world's weather as it
    /// is, so a world that starts in snow is already white.
    now: Option<Eased>,
    drops: Vec<Drop>,
    flakes: Vec<Drop>,
    /// Seconds snowflakes have swayed for.
    clock: f32,
    /// The wind the drops were last moved by, which slants the rain.
    wind: Vec3,
    /// Splashes of raindrops that have just landed.
    splashes: Vec<Splash>,
    /// A fraction of a splash carried over from the last update.
    splash_debt: f32,
    /// Rainwater dripping from the edges of roofs and ledges.
    drips: Vec<Drip>,
    drip_debt: f32,
    /// Last camera position observed by [`add_weather`](crate::Particles::add_weather).
    last_camera: Cell<Option<Vec3>>,
    /// The way that camera looked, level (zero if straight up or down).
    last_heading: Cell<Vec3>,
    /// Cached sky-visibility grid around the camera.
    sky_grid: Option<SkyGrid>,
    /// Whether precipitation & surface occlusion is enabled.
    occlusion_enabled: bool,
    /// Cached manual interior/exterior overrides parsed from `Weather.SurfaceMask`.
    manual_mask_raw: String,
    manual_mask: BTreeMap<(i32, i32), SurfaceMaskKind>,
    /// Cached `Interior` `GraphicsVolume` boxes for precipitation occlusion.
    interiors: Vec<InteriorBox>,
    /// The world's WeatherVolumes, in the order they apply, for
    /// precipitation occlusion.
    volumes: Vec<WeatherVolume>,
    /// What's pressed into the snow round the camera.
    prints: prints::Prints,
    /// Cached GPU weather field built during `update`.
    field: Option<Arc<WeatherField>>,
}

impl WeatherState {
    /// Starts again from the world's weather as it is, such as when Play
    /// starts or stops.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Records the camera for the next sky-visibility update, and so that
    /// raindrops splash where it looks.
    pub(crate) fn note_camera(&self, camera: Camera) {
        if camera.position.is_finite() {
            self.last_camera.set(Some(camera.position));
            let ahead = camera.look_at - camera.position;
            self.last_heading
                .set(Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero());
        }
    }

    /// Eases on by `seconds` toward the world's weather, moves the rain and
    /// snow, updates sky-visibility occlusion via `surfaces`, and presses
    /// what touches the snow into it.
    pub(crate) fn update(
        &mut self,
        world: &World,
        seconds: f32,
        random: &mut Random,
        surfaces: &mut Surfaces,
    ) {
        let target = Target::of(world);
        let now = match self.now {
            None => Eased::settled(&target),
            Some(now) => {
                // The share of a whole change made in `seconds`.
                let step = if target.transition > 0.0 {
                    seconds / target.transition
                } else {
                    1.0
                };
                let toward = |from: f32, to: f32| from + (to - from).clamp(-step, step);
                Eased {
                    rain: toward(now.rain, target.rain),
                    snow: toward(now.snow, target.snow),
                    wetness: toward(now.wetness, target.wetness),
                    cover: toward(now.cover, target.cover),
                    snow_thickness: toward(now.snow_thickness, target.snow_thickness),
                    lightning: toward(now.lightning, target.lightning),
                }
            }
        };
        self.now = Some(now);
        self.occlusion_enabled = target.occlusion;

        if self.manual_mask_raw != target.surface_mask {
            self.manual_mask = parse_surface_mask(&target.surface_mask);
            self.manual_mask_raw = target.surface_mask;
        }

        let wind = global_wind(world);
        self.wind = wind;
        if now.rain > 0.0 {
            fill(&mut self.drops, MOST_DROPS, random);
            for drop in &mut self.drops {
                let velocity = wind - Vec3::Y * RAIN_SPEED * drop.speed;
                drop.position = (drop.position + velocity * seconds).rem_euclid(BOX);
            }
        }
        // The flakes move while snow falls, and while the wind runs lying
        // snow along the ground on their places (spindrift, `sprites`).
        let spindrift =
            now.cover > 0.35 && Vec3::new(wind.x, 0.0, wind.z).length() > SPINDRIFT_WIND.0;
        if now.snow > 0.0 || spindrift {
            fill(&mut self.flakes, MOST_FLAKES, random);
            self.clock += seconds;
            for flake in &mut self.flakes {
                let velocity = flake_velocity(flake, self.clock, wind);
                flake.position = (flake.position + velocity * seconds).rem_euclid(BOX);
            }
        }

        let weather_active =
            now.rain > 0.0 || now.snow > 0.0 || now.wetness > 0.0 || now.cover > 0.0;
        if !weather_active {
            self.prints.clear();
            self.interiors.clear();
            self.volumes.clear();
            self.field = None;
            return;
        }

        // Collect Interior GraphicsVolume boxes for precipitation occlusion.
        self.interiors = collect_interior_boxes(world);
        // And the WeatherVolumes, which say where drops fall before
        // anything else does.
        self.volumes = weather_volumes(world);
        self.update_splashes(seconds, now.rain, random, surfaces);

        // Update sky-visibility grid around the camera when occlusion is enabled.
        let camera = self.last_camera.get().unwrap_or(Vec3::ZERO);
        let origin = [
            (camera.x.floor() as i32) - (FIELD_SIZE as i32 / 2),
            (camera.z.floor() as i32) - (FIELD_SIZE as i32 / 2),
        ];
        if self.occlusion_enabled {
            let grid = self
                .sky_grid
                .get_or_insert_with(|| SkyGrid::new(origin, camera.y));
            grid.recenter(origin, camera.y);
            grid.update(surfaces, &self.prints.passers());
        }

        self.update_drips(seconds, now.rain, random);

        // What touches the snow presses it, and old prints fill in.
        if target.footprints && now.cover > 0.05 {
            let (volumes, manual_mask, interiors) =
                (&self.volumes, &self.manual_mask, &self.interiors);
            let grid = self.sky_grid.as_ref().filter(|_| self.occlusion_enabled);
            let snow = prints::Snow {
                cover: now.cover,
                thickness: now.snow_thickness,
                falling: now.snow,
                fade: target.footprint_fade,
                share: &|ground, top| {
                    snow_share(volumes, manual_mask, interiors, grid, ground, top)
                },
            };
            self.prints.update(world, seconds, camera, &snow, surfaces);
        } else {
            self.prints.clear();
        }

        // Build GPU WeatherField when occlusion, manual masks, or prints are active.
        let prints = self.prints.prints();
        let needs_field =
            self.occlusion_enabled || !self.manual_mask.is_empty() || prints.is_some();
        if needs_field {
            self.field = Some(Arc::new(self.build_field(origin, prints)));
        } else {
            self.field = None;
        }
    }

    /// Whether rain and snow fall at `position` by the WeatherVolumes'
    /// say: the `Precipitation` of the last one (the one that wins) the
    /// point is in, or `None` in none of them.
    fn volume_precipitation(&self, position: Vec3) -> Option<bool> {
        self.volumes
            .iter()
            .rev()
            .find(|volume| volume.contains(position))
            .map(|volume| volume.precipitation)
    }

    /// Ages the splashes and lands `rain`'s new drops on what `surfaces`
    /// finds under them, in front of the camera and nearer it more often.
    /// Drops that would land inside an Interior volume or a cell painted
    /// Interior don't, and a WeatherVolume there decides before either:
    /// none land in one whose `Precipitation` is off.
    fn update_splashes(
        &mut self,
        seconds: f32,
        rain: f32,
        random: &mut Random,
        surfaces: &mut Surfaces,
    ) {
        for splash in &mut self.splashes {
            splash.age += seconds;
        }
        self.splashes.retain(|splash| splash.age < splash.life);
        let Some(camera) = self.last_camera.get().filter(|_| rain > 0.0) else {
            self.splash_debt = 0.0;
            return;
        };
        self.splash_debt += SPLASHES_A_SECOND * rain * seconds;
        let landing = (self.splash_debt as usize).min(SPLASHES_AN_UPDATE);
        self.splash_debt = (self.splash_debt - landing as f32).min(1.0);
        if landing == 0 {
            return;
        }
        let heading = self.last_heading.get();
        let paths: Vec<(Vector3, Vector3)> = (0..landing)
            .map(|_| {
                // Nearer the camera more often, where a splash is seen best,
                // and within the view (anywhere round it when it looks
                // straight up or down).
                let away = 1.5 + (SPLASH_REACH - 1.5) * random.next().powi(2);
                let spread = if heading == Vec3::ZERO {
                    std::f32::consts::PI
                } else {
                    1.05
                };
                let turn = (random.next() * 2.0 - 1.0) * spread;
                let ahead = if heading == Vec3::ZERO {
                    Vec3::X
                } else {
                    heading
                };
                let level = Quat::from_rotation_y(turn) * ahead * away;
                let x = camera.x + level.x;
                let z = camera.z + level.z;
                (
                    Vector3::new(x, camera.y + SKY_RAY_HEIGHT, z),
                    Vector3::new(x, camera.y - SKY_RAY_DEPTH, z),
                )
            })
            .collect();
        for hit in surfaces(None, &paths).into_iter().flatten() {
            let position = Vec3::new(hit.position.x, hit.position.y, hit.position.z);
            let cell = (position.x.floor() as i32, position.z.floor() as i32);
            let sheltered = match self.volume_precipitation(position) {
                Some(falls) => !falls,
                None => {
                    self.manual_mask.get(&cell) == Some(&SurfaceMaskKind::Interior)
                        || self.interiors.iter().any(|b| b.contains(position))
                }
            };
            if sheltered {
                continue;
            }
            self.splashes.push(Splash {
                position,
                age: 0.0,
                life: SPLASH_LIFE * (0.7 + 0.6 * random.next()),
                size: SPLASH_SIZE * (0.7 + 0.6 * random.next()),
            });
        }
    }

    /// Moves the drips on by `seconds` (each splashes where it lands) and
    /// starts `rain`'s new ones at the edges the sky-visibility grid shows:
    /// where one cell's top stands [`DRIP_LEDGE`] or more over the next's.
    fn update_drips(&mut self, seconds: f32, rain: f32, random: &mut Random) {
        let level_wind = Vec3::new(self.wind.x, 0.0, self.wind.z) * 0.12;
        let mut landed = Vec::new();
        for drip in &mut self.drips {
            drip.speed += DRIP_GRAVITY * seconds;
            drip.position += (level_wind - Vec3::Y * drip.speed) * seconds;
            if drip.position.y <= drip.floor {
                landed.push(Vec3::new(drip.position.x, drip.floor, drip.position.z));
            }
        }
        self.drips.retain(|drip| drip.position.y > drip.floor);
        for position in landed {
            self.splashes.push(Splash {
                position,
                age: 0.0,
                life: SPLASH_LIFE * (0.7 + 0.6 * random.next()),
                size: SPLASH_SIZE * (0.6 + 0.5 * random.next()),
            });
        }
        let Some(grid) = self
            .sky_grid
            .as_ref()
            .filter(|_| rain > 0.0 && self.occlusion_enabled)
        else {
            self.drip_debt = 0.0;
            return;
        };
        // Each edge: the higher cell, and which way the drop lies from it.
        let mut edges = Vec::new();
        let size = FIELD_SIZE as i32;
        for z in 0..size {
            for x in 0..size {
                let here = (z * size + x) as usize;
                let top = grid.roof_y[here];
                if !grid.sampled[here] || top <= OPEN_SKY_Y * 0.5 {
                    continue;
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, nz) = (x + dx, z + dz);
                    if nx < 0 || nz < 0 || nx >= size || nz >= size {
                        continue;
                    }
                    let next = (nz * size + nx) as usize;
                    let below = grid.roof_y[next];
                    if grid.sampled[next] && below > OPEN_SKY_Y * 0.5 && top - below >= DRIP_LEDGE {
                        edges.push((x, z, dx, dz, top, below));
                    }
                }
            }
        }
        if edges.is_empty() {
            self.drip_debt = 0.0;
            return;
        }
        self.drip_debt += DRIPS_A_SECOND * rain * seconds * edges.len() as f32;
        let starting = (self.drip_debt as usize)
            .min(DRIPS_AN_UPDATE)
            .min(MOST_DRIPS.saturating_sub(self.drips.len()));
        self.drip_debt = (self.drip_debt - starting as f32).min(1.0);
        for _ in 0..starting {
            let pick = ((random.next() * edges.len() as f32) as usize).min(edges.len() - 1);
            let (x, z, dx, dz, top, below) = edges[pick];
            // Somewhere along the side the two cells share.
            let along = random.next() - 0.5;
            let out = 0.5 + 0.12 * (random.next() - 0.5);
            let cell = Vec3::new(
                (grid.origin[0] + x) as f32 + 0.5,
                0.0,
                (grid.origin[1] + z) as f32 + 0.5,
            );
            let offset = Vec3::new(
                dx as f32 * out + dz as f32 * along,
                0.0,
                dz as f32 * out + dx as f32 * along,
            );
            let position = cell + offset + Vec3::Y * top;
            let sheltered = match self.volume_precipitation(position) {
                Some(falls) => !falls,
                None => self.interiors.iter().any(|b| b.contains(position)),
            };
            if sheltered {
                continue;
            }
            self.drips.push(Drip {
                position,
                speed: 2.0 * random.next(),
                floor: below,
            });
        }
    }

    fn build_field(&mut self, origin: [i32; 2], prints: Option<WeatherPrints>) -> WeatherField {
        let count = FIELD_SIZE * FIELD_SIZE;
        let mut cells = vec![[OPEN_SKY_Y, 0.0, 0.0, 0.0]; count];

        if self.occlusion_enabled
            && let Some(grid) = &self.sky_grid
            && grid.origin == origin
        {
            for (cell, &roof) in cells.iter_mut().zip(grid.roof_y.iter()) {
                cell[0] = roof;
            }
            for (cell, bank) in cells.iter_mut().zip(bank_heights(&grid.roof_y)) {
                cell[3] = bank;
            }
        }

        if !self.manual_mask.is_empty() {
            for (&(wx, wz), &kind) in &self.manual_mask {
                let gx = wx - origin[0];
                let gz = wz - origin[1];
                if gx >= 0 && gz >= 0 && gx < FIELD_SIZE as i32 && gz < FIELD_SIZE as i32 {
                    cells[(gz as usize) * FIELD_SIZE + (gx as usize)][1] = kind.as_shader_value();
                }
            }
        }

        // How deep each cell is pressed on average, for the terrain's
        // corners; the prints' shapes are in the fine map.
        if prints.is_some() {
            self.prints.average_into(origin, FIELD_SIZE, &mut cells);
        }

        WeatherField {
            origin: [origin[0] as f32, origin[1] as f32],
            cell_size: FIELD_CELL_SIZE,
            size: FIELD_SIZE as u32,
            cells,
            prints,
        }
    }

    /// How wet and snowy surfaces are now. `None` before the first update.
    pub(crate) fn look(&self) -> Option<Weather> {
        self.now.map(|now| Weather {
            wetness: now.wetness,
            snow: now.cover,
            snow_thickness: if now.cover > 0.0 {
                now.snow_thickness
            } else {
                0.0
            },
            rain: now.rain,
            snowfall: now.snow,
            lightning: now.lightning,
        })
    }

    /// The latest GPU weather field, if occlusion, manual masks, or prints
    /// are active.
    pub(crate) fn field(&self) -> Option<Arc<WeatherField>> {
        self.field.clone()
    }

    /// True while rain or snow falls, or the weather is still easing, so
    /// a host keeps drawing frames.
    pub(crate) fn is_active(&self, world: &World) -> bool {
        match self.now {
            None => false,
            Some(now) => {
                now.rain > 0.0
                    || now.snow > 0.0
                    || self.prints.is_fading()
                    || now != Eased::settled(&Target::of(world))
            }
        }
    }

    /// True when a precipitation drop at `position` is occluded by a roof,
    /// painted interior cell, or `Interior` `GraphicsVolume`. A
    /// WeatherVolume the drop is in decides before any of them: occluded
    /// if its `Precipitation` is off, and not if it's on, whatever the
    /// roof says.
    fn is_drop_occluded(&self, position: Vec3) -> bool {
        if let Some(falls) = self.volume_precipitation(position) {
            return !falls;
        }
        if !self.occlusion_enabled && self.manual_mask.is_empty() && self.interiors.is_empty() {
            return false;
        }
        if self.interiors.iter().any(|b| b.contains(position)) {
            return true;
        }
        let cell_key = (position.x.floor() as i32, position.z.floor() as i32);
        if let Some(&kind) = self.manual_mask.get(&cell_key) {
            return kind == SurfaceMaskKind::Interior;
        }
        if self.occlusion_enabled
            && let Some(grid) = &self.sky_grid
            && let Some(roof_y) = grid.roof_at(position.x, position.z)
            && roof_y > OPEN_SKY_Y * 0.5
            && position.y < roof_y
        {
            return true;
        }
        false
    }

    /// The raindrops and snowflakes to draw around `camera`: as many of
    /// them as how hard each falls now, fading out toward the box's sides
    /// and right at the camera, and culled when occluded by roofs or
    /// interior zones. A raindrop's streak lies along the way it falls as
    /// `camera` sees it, so wind slants the rain.
    pub(crate) fn sprites(&self, camera: Camera) -> Vec<Sprite> {
        let Some(now) = self.now else {
            return Vec::new();
        };
        let eye = camera.position;
        // The camera's right and up, as sprites are turned in the view.
        let forward = (camera.look_at - eye).normalize_or(Vec3::NEG_Z);
        let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(forward);
        let mut sprites = Vec::new();
        let rain_color = [0.8, 0.84, 0.9];
        let streak = rain_streak();
        let rain = Look {
            // About as far as a drop falls while a film camera's shutter
            // is open.
            size: 0.95,
            color: rain_color,
            // Rain is seen as a veil of many faint streaks, each a
            // different length and strength (below), not as lines.
            opacity: 0.45,
            texture: streak.clone(),
            near: RAIN_NEAR,
            streak: true,
        };
        let snow = Look {
            size: FLAKE_SIZE,
            color: [1.0; 3],
            opacity: 0.85,
            texture: snow_flakes(),
            near: SNOW_NEAR,
            streak: false,
        };
        for (drops, amount, look) in [
            (&self.drops, now.rain, rain),
            (&self.flakes, now.snow, snow),
        ] {
            let shown = (amount * drops.len() as f32).round() as usize;
            for drop in &drops[..shown.min(drops.len())] {
                let position = around(drop.position, eye);
                if self.is_drop_occluded(position) {
                    continue;
                }
                let out = ((position - eye) / (BOX * 0.5)).length().min(1.0);
                let distance = (position - eye).length();
                let near = ((distance - look.near.0) / (look.near.1 - look.near.0)).clamp(0.0, 1.0);
                // Each lies along the way it falls as the camera sees it: a
                // streak of rain, and a flake's little blur.
                let velocity = if look.streak {
                    self.wind - Vec3::Y * RAIN_SPEED * drop.speed
                } else {
                    flake_velocity(drop, self.clock, self.wind)
                };
                let rotation_degrees = velocity.dot(right).atan2(-velocity.dot(up)).to_degrees();
                // Faster drops leave longer streaks. Flakes differ more in
                // size than in speed: most are small, a few are clumps.
                let turns = drop.phase / std::f32::consts::TAU;
                let (size, faint, frame) = if look.streak {
                    // Longer the faster it falls and the nearer it passes
                    // (its blur crosses more of the picture), and each as
                    // strong as its own size and the light catch it.
                    let pass = 0.7 + 0.6 * (turns * 13.0).fract();
                    (drop.speed * pass, 0.35 + 0.65 * (turns * 7.0).fract(), None)
                } else {
                    // Out of focus near the lens: wider, and fainter by as
                    // much. No two alike: one of the pictures, by its phase.
                    let blur = 1.0
                        + (FLAKE_BLUR.1 - 1.0) * (1.0 - distance / FLAKE_BLUR.0).clamp(0.0, 1.0);
                    let pictures = FLAKE_GRID * FLAKE_GRID;
                    (
                        (0.6 + 1.2 * turns.powi(3)) * blur,
                        (0.55 + 0.45 * (turns * 7.0).fract()) / blur,
                        Some(FlipbookFrame {
                            grid: FLAKE_GRID,
                            frame: ((turns * 53.0).fract() * pictures as f32) as u32 % pictures,
                        }),
                    )
                };
                sprites.push(Sprite {
                    position,
                    size: look.size * size,
                    rotation_degrees,
                    color: look.color,
                    opacity: look.opacity * faint * (1.0 - out * out) * near * near,
                    brightness: 1.0,
                    light_emission: 0.0,
                    light_influence: WEATHER_LIGHT,
                    texture: Some(look.texture.clone()),
                    frame,
                });
            }
        }
        // Spindrift: where snow lies and the wind blows, streaks of it run
        // along the ground, low and faint, the way the wind goes. They ride
        // on the flakes' own places (from the far end of the list, so they
        // don't depend on how hard it snows) set down on what's under them.
        let level = Vec3::new(self.wind.x, 0.0, self.wind.z);
        let blowing = ((level.length() - SPINDRIFT_WIND.0) / (SPINDRIFT_WIND.1 - SPINDRIFT_WIND.0))
            .clamp(0.0, 1.0);
        let lying = ((now.cover - 0.35) / 0.5).clamp(0.0, 1.0);
        let drift = lying * blowing * blowing * (3.0 - 2.0 * blowing);
        if let (true, Some(grid)) = (drift > 0.0, &self.sky_grid) {
            let depth = now.snow_thickness * lying;
            let rotation_degrees = level.dot(right).atan2(-level.dot(up)).to_degrees();
            let shown = ((0.25 + 0.75 * drift) * MOST_SPINDRIFT as f32) as usize;
            for flake in self.flakes.iter().rev().take(shown) {
                let over = around(flake.position, eye);
                let Some(ground) = grid.roof_at(over.x, over.z) else {
                    continue;
                };
                if ground <= OPEN_SKY_Y * 0.5 {
                    continue;
                }
                let turns = flake.phase / std::f32::consts::TAU;
                let height = 0.06 + SPINDRIFT_HEIGHT * (turns * 29.0).fract().powi(2);
                let position = Vec3::new(over.x, ground + depth + height, over.z);
                if self.is_drop_occluded(position) {
                    continue;
                }
                let flat = Vec3::new(position.x - eye.x, 0.0, position.z - eye.z);
                let out = (flat / (BOX * 0.5)).length().min(1.0);
                let distance = (position - eye).length();
                let near = ((distance - RAIN_NEAR.0) / (RAIN_NEAR.1 - RAIN_NEAR.0)).clamp(0.0, 1.0);
                sprites.push(Sprite {
                    position,
                    size: 1.1 + 1.7 * (turns * 13.0).fract(),
                    rotation_degrees,
                    color: [1.0; 3],
                    opacity: 0.2
                        * drift.sqrt()
                        * (0.35 + 0.65 * (turns * 7.0).fract())
                        * (1.0 - out * out)
                        * near,
                    brightness: 1.0,
                    light_emission: 0.0,
                    light_influence: WEATHER_LIGHT,
                    texture: Some(streak.clone()),
                    frame: None,
                });
            }
        }
        // Drips: short streaks, longer the faster they fall, upright but
        // for the little the wind carries them.
        for drip in &self.drips {
            let length = (0.12 + drip.speed * 0.035).min(DRIP_LENGTH);
            let position = drip.position + Vec3::Y * (length * 0.5);
            let distance = (position - eye).length();
            let near = ((distance - RAIN_NEAR.0) / (RAIN_NEAR.1 - RAIN_NEAR.0)).clamp(0.0, 1.0);
            let far = 1.0 - (distance / (BOX.x * 0.5)).clamp(0.0, 1.0).powi(2);
            sprites.push(Sprite {
                position,
                size: length,
                rotation_degrees: 0.0,
                color: rain_color,
                opacity: 0.75 * near * far,
                brightness: 1.0,
                light_emission: 0.0,
                light_influence: WEATHER_LIGHT,
                texture: Some(streak.clone()),
                frame: None,
            });
        }
        // Splashes: a crown of spray standing on where each drop landed,
        // widening and thinning as it's thrown up.
        let picture = rain_splash();
        for splash in &self.splashes {
            let age = (splash.age / splash.life).clamp(0.0, 1.0);
            let size = splash.size * (0.45 + 0.75 * age);
            let position = splash.position + Vec3::Y * (size * 0.5);
            let distance = (position - eye).length();
            let far = 1.0 - (distance / SPLASH_REACH).clamp(0.0, 1.0).powi(2);
            let near = ((distance - RAIN_NEAR.0) / (RAIN_NEAR.1 - RAIN_NEAR.0)).clamp(0.0, 1.0);
            sprites.push(Sprite {
                position,
                size,
                rotation_degrees: 0.0,
                color: rain_color,
                opacity: 0.42 * (1.0 - age).powf(1.5) * far * near,
                brightness: 1.0,
                light_emission: 0.0,
                light_influence: WEATHER_LIGHT,
                texture: Some(picture.clone()),
                frame: None,
            });
        }
        sprites
    }
}

fn collect_interior_boxes(world: &World) -> Vec<InteriorBox> {
    let mut out = Vec::new();
    for id in world.descendants(world.workspace()) {
        let Some(instance) = world.get(id) else {
            continue;
        };
        if instance.class() != ClassName::GraphicsVolume {
            continue;
        }
        if instance.property("Enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let has_interior_fog = instance.children().iter().any(|&cid| {
            world.get(cid).is_some_and(|child| {
                child.class() == ClassName::FogEffect
                    && child.property("Enabled").and_then(Value::as_bool) != Some(false)
                    && child.property("Interior") == Some(&Value::Bool(true))
            })
        });
        if !has_interior_fog {
            continue;
        }
        let Some(Value::Vector3(pos)) = instance.property("Position") else {
            continue;
        };
        let Some(Value::Vector3(size)) = instance.property("Size") else {
            continue;
        };
        let rot = match instance.property("Orientation") {
            Some(Value::Vector3(o)) => rotation_from_orientation(*o),
            _ => Quat::IDENTITY,
        };
        out.push(InteriorBox {
            center: Vec3::new(pos.x, pos.y, pos.z),
            inv_rotation: rot.inverse(),
            half_extent: Vec3::new(size.x.abs() * 0.5, size.y.abs() * 0.5, size.z.abs() * 0.5),
        });
    }
    out
}

/// How raindrops or snowflakes look.
struct Look {
    size: f32,
    color: [f32; 3],
    opacity: f32,
    texture: Arc<dyn TextureSource>,
    /// Where it starts to show and where it's fully shown, in studs from
    /// the camera.
    near: (f32, f32),
    /// Whether its picture is a streak, laid along its fall.
    streak: bool,
}

/// How far what stands in each cell of a field of roof heights (`FIELD_SIZE`
/// a side, [`OPEN_SKY_Y`] for none) rises over the ground round it, in studs
/// and at most [`BANK_MOST`], spread a couple of cells outward: where the
/// ground's snow drifts against it (lighting.wgsl's `SNOW_BANK`). The ground
/// round a cell is the lowest height within [`BANK_REACH`] cells of it, so
/// the middle of a wide roof banks nothing, and its edge does.
fn bank_heights(roof_y: &[f32]) -> Vec<f32> {
    let side = FIELD_SIZE as i32;
    let at = |x: i32, z: i32| roof_y[(z.clamp(0, side - 1) * side + x.clamp(0, side - 1)) as usize];
    let known = |height: f32| height > OPEN_SKY_Y * 0.5;
    let mut rise = vec![0.0f32; roof_y.len()];
    for z in 0..side {
        for x in 0..side {
            let here = at(x, z);
            if !known(here) {
                continue;
            }
            let mut ground = here;
            for dz in -BANK_REACH..=BANK_REACH {
                for dx in -BANK_REACH..=BANK_REACH {
                    let near = at(x + dx, z + dz);
                    if known(near) {
                        ground = ground.min(near);
                    }
                }
            }
            // A step under a quarter of a stud is the ground's own slope.
            rise[(z * side + x) as usize] = (here - ground - 0.25).clamp(0.0, BANK_MOST);
        }
    }
    // Spread: each cell takes its neighbours' rise, falling off with the
    // distance, so the drift climbs over a couple of studs.
    let mut bank = vec![0.0f32; roof_y.len()];
    for z in 0..side {
        for x in 0..side {
            let mut most: f32 = 0.0;
            for dz in -2..=2i32 {
                for dx in -2..=2i32 {
                    let (nx, nz) = (x + dx, z + dz);
                    if nx < 0 || nz < 0 || nx >= side || nz >= side {
                        continue;
                    }
                    let away = ((dx * dx + dz * dz) as f32).sqrt();
                    most = most.max(rise[(nz * side + nx) as usize] * (1.0 - away / 3.0).max(0.0));
                }
            }
            bank[(z * side + x) as usize] = most;
        }
    }
    bank
}

/// How a snowflake moves at `clock` seconds: down, slower the lighter it
/// is, carried by the `wind`, and wandering as the air eddies round it: a
/// slow drift one way and a quicker flutter across it, each flake in its
/// own time, which also checks and quickens its fall.
fn flake_velocity(flake: &Drop, clock: f32, wind: Vec3) -> Vec3 {
    let slow = clock * 0.8 + flake.phase;
    let quick = clock * (1.9 + 0.6 * flake.speed) + flake.phase * 2.7;
    let drift = Vec3::new(slow.sin(), 0.0, slow.cos()) * SNOW_SWAY
        + Vec3::new(quick.cos(), 0.0, -quick.sin()) * (SNOW_SWAY * 0.45);
    let fall = SNOW_SPEED * flake.speed * (1.0 + 0.25 * quick.sin());
    wind - Vec3::Y * fall + drift
}

/// Gives `drops` its `most` drops, spread through the box, the first time
/// they're needed.
fn fill(drops: &mut Vec<Drop>, most: usize, random: &mut Random) {
    while drops.len() < most {
        drops.push(Drop {
            position: Vec3::new(random.next(), random.next(), random.next()) * BOX,
            speed: 0.8 + 0.4 * random.next(),
            phase: random.next() * std::f32::consts::TAU,
        });
    }
}

/// The copy of a drop at `position` (inside [`BOX`]) that's in the box
/// centered on `camera`.
fn around(position: Vec3, camera: Vec3) -> Vec3 {
    let corner = camera - BOX * 0.5;
    corner + (position - corner).rem_euclid(BOX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::SurfaceHit;
    use axcenity_core::{EnumItem, InstanceId};

    fn no_surfaces(
        _skip: Option<InstanceId>,
        paths: &[(Vector3, Vector3)],
    ) -> Vec<Option<SurfaceHit>> {
        vec![None; paths.len()]
    }

    /// A camera at `position`, looking north.
    fn view(position: Vec3) -> Camera {
        Camera {
            position,
            look_at: position + Vec3::NEG_Z,
            fov_y_degrees: 50.0,
        }
    }

    fn falling(item: &str) -> Value {
        Value::EnumItem(EnumItem {
            enum_name: "Precipitation".to_owned(),
            item: item.to_owned(),
        })
    }

    fn world_with_weather() -> (World, InstanceId) {
        let mut world = World::new();
        let weather = world.create(ClassName::Weather).unwrap();
        world.set_parent(weather, Some(world.lighting())).unwrap();
        (world, weather)
    }

    fn set(world: &mut World, id: InstanceId, name: &str, value: Value) {
        world.set_property(id, name, value).unwrap();
    }

    #[test]
    fn a_new_world_is_dry_and_a_weathers_rain_wets_it() {
        assert!(settled(&World::new()).is_clear());
        let (mut world, weather) = world_with_weather();
        // Rain at 0.5 by default.
        assert_eq!(
            settled(&world),
            Weather {
                wetness: 0.5,
                snow: 0.0,
                snow_thickness: 0.0,
                rain: 0.5,
                snowfall: 0.0,
                lightning: 0.0,
            }
        );
        set(&mut world, weather, "Wetness", Value::Number(0.8));
        assert_eq!(settled(&world).wetness, 0.8, "wetter than the rain");
        set(&mut world, weather, "Precipitation", falling("Snow"));
        set(&mut world, weather, "Intensity", Value::Number(3.0));
        assert_eq!(
            settled(&world),
            Weather {
                wetness: 0.8,
                snow: 1.0,
                snow_thickness: DEFAULT_SNOW_THICKNESS,
                rain: 0.0,
                snowfall: 1.0,
                lightning: 0.0,
            },
            "snow covers, held to 1"
        );
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "Wetness", Value::Number(0.0));
        set(&mut world, weather, "SnowCover", Value::Number(0.25));
        assert_eq!(settled(&world).snow, 0.25, "old snow lies");
        set(&mut world, weather, "Enabled", Value::Bool(false));
        assert!(settled(&world).is_clear(), "disabling Weather clears it");
        world.destroy(weather).unwrap();
        assert!(settled(&world).is_clear());
    }

    #[test]
    fn rain_wets_surfaces_over_the_transition_and_they_dry_after() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        set(&mut world, weather, "TransitionTime", Value::Number(4.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let mut surfaces = no_surfaces;
        state.update(&world, 0.1, &mut random, &mut surfaces);
        assert!(state.look().unwrap().is_clear(), "starts as it is: dry");
        assert!(!state.is_active(&world), "clear and settled: nothing to do");

        set(&mut world, weather, "Precipitation", falling("Rain"));
        let mut wetness = Vec::new();
        for _ in 0..5 {
            state.update(&world, 1.0, &mut random, &mut surfaces);
            wetness.push(state.look().unwrap().wetness);
        }
        assert_eq!(wetness, [0.25, 0.5, 0.75, 1.0, 1.0]);
        assert!(state.is_active(&world), "rain falls");

        set(&mut world, weather, "Precipitation", falling("None"));
        state.update(&world, 2.0, &mut random, &mut surfaces);
        assert_eq!(state.look().unwrap().wetness, 0.5, "drying");
        state.update(&world, 2.0, &mut random, &mut surfaces);
        assert!(state.look().unwrap().is_clear(), "dry again");
        assert!(!state.is_active(&world));
        assert!(state.sprites(view(Vec3::ZERO)).is_empty(), "no rain left");

        // TransitionTime 0 changes at once.
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        set(&mut world, weather, "Precipitation", falling("Snow"));
        state.update(&world, 0.01, &mut random, &mut surfaces);
        assert_eq!(state.look().unwrap().snow, 1.0);
    }

    #[test]
    fn rain_and_snow_fall_around_the_camera_blown_by_the_wind() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let mut surfaces = no_surfaces;
        state.update(&world, 0.0, &mut random, &mut surfaces);
        let camera = Vec3::new(1000.0, 50.0, -3000.0);
        let drops = state.sprites(view(camera));
        assert_eq!(drops.len(), MOST_DROPS);
        assert!(
            drops
                .iter()
                .all(|drop| ((drop.position - camera).abs() - BOX * 0.5).max_element() <= 1e-3),
            "every drop in the box around the camera"
        );

        // They fall, and the wind blows them along.
        let workspace = world.workspace();
        set(
            &mut world,
            workspace,
            "GlobalWind",
            Value::Vector3(Vector3::new(10.0, 0.0, 0.0)),
        );
        let before = state.drops.clone();
        state.update(&world, 0.1, &mut random, &mut surfaces);
        let moved = state.drops[0].position - before[0].position;
        let fall = RAIN_SPEED * before[0].speed * 0.1;
        assert!((moved.x - 1.0).abs() < 1e-3, "{moved}");
        assert!(
            (moved.y + fall).abs() < 1e-3 || (moved.y + fall - BOX.y).abs() < 1e-3,
            "{moved}"
        );

        // A streak lies along its drop's fall: seen looking north, an east
        // wind carries the drop right as it falls, so its foot leans right.
        let leaning = state.sprites(view(camera));
        let lean = 10.0_f32.atan2(RAIN_SPEED * before[0].speed).to_degrees();
        assert!(
            (leaning[0].rotation_degrees - lean).abs() < 1e-3,
            "{} for {lean}",
            leaning[0].rotation_degrees
        );
        // Seen along the wind, it falls straight down the picture.
        let downwind = Camera {
            look_at: camera + Vec3::X,
            ..view(camera)
        };
        assert!(state.sprites(downwind)[0].rotation_degrees.abs() < 1e-3);
        // None is drawn right at the lens.
        assert!(
            leaning
                .iter()
                .filter(|drop| (drop.position - camera).length() <= RAIN_NEAR.0)
                .all(|drop| drop.opacity == 0.0)
        );
        assert!(leaning.iter().any(|drop| drop.opacity > 0.3));

        // Half as hard, half as many; snow in place of rain once it's eased.
        set(&mut world, weather, "Intensity", Value::Number(0.5));
        set(&mut world, weather, "Precipitation", falling("Snow"));
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        state.update(&world, 0.1, &mut random, &mut surfaces);
        assert_eq!(state.sprites(view(camera)).len(), MOST_FLAKES / 2);
    }

    #[test]
    fn raindrops_splash_where_they_land_in_front_of_the_camera() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        // Looking north (-Z) over a floor at y = 2.
        state.note_camera(view(camera));
        let mut floor = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(from.x, 2.0, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        state.update(&world, 0.0, &mut random, &mut floor);
        assert!(state.splashes.is_empty(), "no time, no drops landed");
        state.update(&world, 0.03, &mut random, &mut floor);
        let landed = state.splashes.len();
        assert!(
            (1..=SPLASHES_AN_UPDATE).contains(&landed),
            "{landed} splashes"
        );
        assert!(
            state.splashes.iter().all(|splash| {
                let level = splash.position - camera;
                splash.position.y == 2.0
                    && level.z < 0.0
                    && Vec3::new(level.x, 0.0, level.z).length() <= SPLASH_REACH + 1e-3
            }),
            "each stands on the floor, ahead of the camera, within reach"
        );
        // Each is drawn after the drops, as a sprite standing on where it
        // landed (the drops under the floor aren't drawn).
        let sprites = state.sprites(view(camera));
        assert!(sprites.len() > landed, "{} sprites", sprites.len());
        assert!(
            sprites[sprites.len() - landed..]
                .iter()
                .all(|splash| splash.position.y > 2.0 && splash.position.y < 2.0 + SPLASH_SIZE)
        );
        // They're over in a moment, and none land once the rain stops.
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "Wetness", Value::Number(1.0));
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        state.update(&world, SPLASH_LIFE * 1.5, &mut random, &mut floor);
        assert!(state.splashes.is_empty(), "the splashes are over");
        state.update(&world, 0.03, &mut random, &mut floor);
        assert!(state.splashes.is_empty(), "no rain, no splashes");
    }

    #[test]
    fn rain_drips_from_the_edge_of_a_roof_and_splashes_below_it() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        state.note_camera(view(camera));
        // A roof at y = 12 over everything west of x = 0, ground at y = 0 east of it.
        let mut eave = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(
                            from.x,
                            if from.x < 0.0 { 12.0 } else { 0.0 },
                            from.z,
                        ),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        // The grid is read first (512 cells an update), then drips start.
        for _ in 0..10 {
            state.update(&world, 0.016, &mut random, &mut eave);
        }
        assert!(!state.drips.is_empty(), "drips fall from the roof's edge");
        assert!(state.drips.len() <= MOST_DRIPS);
        assert!(
            state.drips.iter().all(|drip| {
                drip.position.x.abs() < 0.75 && drip.position.y <= 12.0 && drip.floor == 0.0
            }),
            "each started at the edge, under the roof's height, bound for the ground"
        );
        // They gain speed as they fall, land within a couple of seconds and splash there.
        let fastest = |state: &WeatherState| {
            state
                .drips
                .iter()
                .map(|drip| drip.speed)
                .fold(0.0, f32::max)
        };
        let before = fastest(&state);
        state.update(&world, 0.1, &mut random, &mut eave);
        assert!(fastest(&state) > before + 1.0);
        state.splashes.clear();
        for _ in 0..20 {
            state.update(&world, 0.05, &mut random, &mut eave);
        }
        assert!(
            state
                .splashes
                .iter()
                .any(|splash| splash.position.y == 0.0 && splash.position.x.abs() < 1.0),
            "a drip splashed on the ground under the edge"
        );
        // No rain, no new drips; the last of them land.
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "Wetness", Value::Number(1.0));
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        for _ in 0..40 {
            state.update(&world, 0.05, &mut random, &mut eave);
        }
        assert!(state.drips.is_empty(), "the drips stopped with the rain");
    }

    #[test]
    fn roof_occludes_rain_drops_below_it_and_populates_weather_field() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        state.note_camera(view(camera));

        // A solid roof at y = 12 covering everything.
        let mut roof_surfaces = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(from.x, 12.0, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        // 8 updates populate all 64x64 cells (512 rays per frame).
        for _ in 0..8 {
            state.update(&world, 0.016, &mut random, &mut roof_surfaces);
        }
        let drops = state.sprites(view(camera));
        assert!(
            drops.len() < MOST_DROPS,
            "drops below the roof are occluded"
        );
        assert!(
            drops.iter().all(|drop| drop.position.y >= 12.0),
            "no raindrop passes underneath the y=12 roof"
        );
        let field = state.field().expect("weather field populated");
        assert!(field.is_valid());
        assert!(
            field.cells.iter().all(|c| (c[0] - 12.0).abs() < 1e-3),
            "every cell records roof_y = 12.0"
        );
    }

    #[test]
    fn lightning_is_read_from_the_weather_and_stops_with_it() {
        let (mut world, weather) = world_with_weather();
        assert_eq!(settled(&world).lightning, 0.0, "none by default");
        set(&mut world, weather, "Lightning", Value::Number(0.4));
        assert_eq!(settled(&world).lightning, 0.4);
        set(&mut world, weather, "Lightning", Value::Number(7.0));
        assert_eq!(settled(&world).lightning, 1.0, "held to 1");
        set(&mut world, weather, "Enabled", Value::Bool(false));
        assert_eq!(settled(&world).lightning, 0.0);
    }

    #[test]
    fn wind_runs_spindrift_along_lying_snow() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "SnowCover", Value::Number(1.0));
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        let camera = Vec3::new(0.0, 5.0, 0.0);
        // Level ground at y = 2 everywhere.
        let mut ground = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(from.x, 2.0, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        let mut streaks = |world: &World| {
            let mut random = Random::default();
            let mut state = WeatherState::default();
            state.note_camera(view(camera));
            for _ in 0..8 {
                state.update(world, 0.016, &mut random, &mut ground);
            }
            state.sprites(view(camera))
        };
        assert!(streaks(&world).is_empty(), "none in still air");

        let wind = |x: f32| Value::Vector3(Vector3::new(x, 0.0, 0.0));
        world
            .set_property(world.workspace(), "GlobalWind", wind(20.0))
            .unwrap();
        let blown = streaks(&world);
        assert!(blown.len() > MOST_SPINDRIFT / 2, "{} streaks", blown.len());
        let depth = DEFAULT_SNOW_THICKNESS;
        assert!(
            blown.iter().all(|streak| {
                let over = streak.position.y - 2.0 - depth;
                (0.0..=SPINDRIFT_HEIGHT + 0.1).contains(&over)
            }),
            "they run just over the snow"
        );

        // No snow lying, none to blow.
        set(&mut world, weather, "SnowCover", Value::Number(0.0));
        assert!(streaks(&world).is_empty());
    }

    /// A WeatherVolume in the Workspace, `size` studs around `at`.
    fn weather_volume(world: &mut World, at: [f32; 3], size: [f32; 3]) -> InstanceId {
        let volume = world.create(ClassName::WeatherVolume).unwrap();
        world.set_parent(volume, Some(world.workspace())).unwrap();
        let vector = |[x, y, z]: [f32; 3]| Value::Vector3(Vector3::new(x, y, z));
        set(world, volume, "Position", vector(at));
        set(world, volume, "Size", vector(size));
        volume
    }

    #[test]
    fn weather_volumes_are_read_with_their_values_in_the_order_they_apply() {
        let mut world = World::new();
        assert!(weather_volumes(&world).is_empty());

        // As made, a shelter: dry, bare, and no drops.
        let shelter = weather_volume(&mut world, [10.0, 6.0, -4.0], [20.0, 12.0, 8.0]);
        let [found] = weather_volumes(&world)[..] else {
            panic!("one volume");
        };
        assert!(!found.precipitation);
        let made = found.weather_box;
        assert_eq!((made.wetness, made.snow, made.blend), (0.0, 0.0, 1.0));
        assert_eq!(made.half_size, Vec3::new(10.0, 6.0, 4.0));
        // Its middle is the box's origin, and a corner is half its size out.
        let middle = made.to_box.transform_point3(Vec3::new(10.0, 6.0, -4.0));
        assert!(middle.length() < 1e-4, "{middle}");
        let corner = made.to_box.transform_point3(Vec3::new(20.0, 12.0, 0.0));
        assert!((corner - made.half_size).length() < 1e-4, "{corner}");
        assert!(found.contains(Vec3::new(19.0, 11.0, -1.0)));
        assert!(!found.contains(Vec3::new(21.0, 6.0, -4.0)));

        // Its own values, each share held to 0 to 2.
        set(&mut world, shelter, "Precipitation", Value::Bool(true));
        set(&mut world, shelter, "Snow", Value::Number(1.5));
        set(&mut world, shelter, "Wetness", Value::Number(7.0));
        set(&mut world, shelter, "BlendDistance", Value::Number(3.0));
        let found = weather_volumes(&world)[0];
        assert!(found.precipitation);
        let read = found.weather_box;
        assert_eq!((read.wetness, read.snow, read.blend), (2.0, 1.5, 3.0));
        set(&mut world, shelter, "Snow", Value::Number(-1.0));
        set(&mut world, shelter, "Wetness", Value::Number(f64::NAN));
        set(&mut world, shelter, "BlendDistance", Value::Number(-2.0));
        let read = weather_volumes(&world)[0].weather_box;
        assert_eq!((read.wetness, read.snow, read.blend), (0.0, 0.0, 0.0));

        // Turned a quarter round, its long side lies along z.
        set(
            &mut world,
            shelter,
            "Orientation",
            Value::Vector3(Vector3::new(0.0, 90.0, 0.0)),
        );
        let turned = weather_volumes(&world)[0];
        assert!(turned.contains(Vec3::new(10.0, 6.0, 5.0)));
        assert!(!turned.contains(Vec3::new(19.0, 6.0, -4.0)));

        // Sorted by Priority, Explorer order between equal ones; the
        // disabled one is left out, and so is one outside the Workspace.
        let snow = |world: &mut World, share: f64, priority: f64| {
            let volume = weather_volume(world, [0.0; 3], [4.0; 3]);
            set(world, volume, "Snow", Value::Number(share));
            set(world, volume, "Priority", Value::Number(priority));
            volume
        };
        set(&mut world, shelter, "Snow", Value::Number(0.1));
        set(&mut world, shelter, "Priority", Value::Number(1.0));
        snow(&mut world, 0.2, 5.0);
        snow(&mut world, 0.3, 0.0);
        let off = snow(&mut world, 0.4, 9.0);
        snow(&mut world, 0.5, 1.0);
        snow(&mut world, 0.6, -2.0);
        let stored = snow(&mut world, 0.7, 3.0);
        set(&mut world, off, "Enabled", Value::Bool(false));
        world
            .set_parent(stored, Some(world.replicated_storage()))
            .unwrap();
        let order: Vec<f32> = weather_volumes(&world)
            .iter()
            .map(|volume| volume.weather_box.snow)
            .collect();
        assert_eq!(order, [0.6, 0.3, 0.1, 0.5, 0.2]);

        // The draw list carries the same boxes in the same order.
        let list = crate::build_draw_list(
            &world,
            &axcenity_assets::Assets::default(),
            view(Vec3::new(0.0, 5.0, 0.0)),
        );
        let drawn: Vec<f32> = list.weather_boxes.iter().map(|b| b.snow).collect();
        assert_eq!(drawn, order);
    }

    #[test]
    fn a_weather_volume_says_where_rain_falls_whatever_the_roof_says() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        let rain_in = |world: &World, surfaces: &mut Surfaces, random: &mut Random| {
            let mut state = WeatherState::default();
            state.note_camera(view(camera));
            // 8 updates sample every cell of the sky-visibility grid.
            for _ in 0..8 {
                state.update(world, 0.016, random, surfaces);
            }
            state
        };
        // West of x = 0 a volume that keeps the rain out, 20 studs high
        // from the ground.
        let dry = weather_volume(&mut world, [-20.0, 10.0, 0.0], [40.0, 20.0, 80.0]);
        let in_the_dry = |p: Vec3| p.x < 0.0 && (0.0..20.0).contains(&p.y);

        // Under open sky: no drop in the volume, and rain all round it.
        let state = rain_in(&world, &mut no_surfaces, &mut random);
        let drops = state.sprites(view(camera));
        assert!(!drops.is_empty());
        assert!(
            drops.iter().all(|drop| !in_the_dry(drop.position)),
            "no rain falls in a volume whose Precipitation is off"
        );
        assert!(drops.iter().any(|drop| drop.position.x > 0.0));
        assert!(state.is_drop_occluded(Vec3::new(-5.0, 3.0, 0.0)));
        assert!(!state.is_drop_occluded(Vec3::new(5.0, 3.0, 0.0)));

        // Disabled, it changes nothing.
        set(&mut world, dry, "Enabled", Value::Bool(false));
        let state = rain_in(&world, &mut no_surfaces, &mut random);
        assert!(!state.is_drop_occluded(Vec3::new(-5.0, 3.0, 0.0)));
        assert!(
            state
                .sprites(view(camera))
                .iter()
                .any(|drop| in_the_dry(drop.position))
        );

        // A roof at y = 12 over everything keeps the rain out from under
        // it, but for a volume there whose Precipitation is on: an
        // open-sided shed the wind blows into, east of x = 4.
        let mut roof = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(from.x, 12.0, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        let shed = weather_volume(&mut world, [14.0, 5.0, 0.0], [20.0, 10.0, 80.0]);
        set(&mut world, shed, "Precipitation", Value::Bool(true));
        let in_the_shed = |p: Vec3| p.x > 4.0 && p.x < 24.0 && (0.0..10.0).contains(&p.y);
        let state = rain_in(&world, &mut roof, &mut random);
        assert!(state.is_drop_occluded(Vec3::new(0.0, 3.0, 0.0)));
        assert!(!state.is_drop_occluded(Vec3::new(10.0, 3.0, 0.0)));
        let drops = state.sprites(view(camera));
        assert!(
            drops.iter().any(|drop| in_the_shed(drop.position)),
            "rain falls in the shed, under the roof"
        );
        assert!(
            drops
                .iter()
                .all(|drop| drop.position.y >= 12.0 || in_the_shed(drop.position)),
            "and nowhere else under it"
        );

        // Where the two overlap the higher Priority decides, and equal
        // ones in Explorer order: the later.
        set(&mut world, dry, "Enabled", Value::Bool(true));
        set(
            &mut world,
            dry,
            "Position",
            Value::Vector3(Vector3::new(14.0, 10.0, 0.0)),
        );
        let state = rain_in(&world, &mut roof, &mut random);
        assert!(
            !state.is_drop_occluded(Vec3::new(10.0, 3.0, 0.0)),
            "the shed is the later of two equal ones"
        );
        set(&mut world, dry, "Priority", Value::Number(1.0));
        let state = rain_in(&world, &mut roof, &mut random);
        assert!(state.is_drop_occluded(Vec3::new(10.0, 3.0, 0.0)));
    }

    #[test]
    fn raindrops_dont_splash_in_a_weather_volume_that_keeps_them_out() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        // Ground at y = 0 everywhere.
        let mut ground = |_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, _)| {
                    Some(SurfaceHit {
                        position: Vector3::new(from.x, 0.0, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        let mut splashes = |world: &World| {
            let mut state = WeatherState::default();
            state.note_camera(view(camera));
            for _ in 0..20 {
                state.update(world, 0.016, &mut random, &mut ground);
            }
            state.splashes.len()
        };
        assert!(splashes(&world) > 0, "rain splashes on open ground");
        // A dry volume over all the ground the camera sees.
        let dry = weather_volume(&mut world, [0.0, 0.0, 0.0], [200.0, 10.0, 200.0]);
        assert_eq!(splashes(&world), 0, "none land in the volume");
        set(&mut world, dry, "Precipitation", Value::Bool(true));
        assert!(splashes(&world) > 0, "they land where its rain falls");
    }

    #[test]
    fn surface_mask_parses_formats_and_paints_cleanly() {
        let mut mask = BTreeMap::new();
        assert!(paint_surface_mask(
            &mut mask,
            0.5,
            0.5,
            1.2,
            Some(SurfaceMaskKind::Interior)
        ));
        assert_eq!(mask.get(&(0, 0)), Some(&SurfaceMaskKind::Interior));
        let encoded = format_surface_mask(&mask);
        let decoded = parse_surface_mask(&encoded);
        assert_eq!(decoded, mask);

        assert!(paint_surface_mask(&mut mask, 0.5, 0.5, 2.0, None));
        assert!(mask.is_empty());
    }
}
