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
//! occludes falling drops against solid roofs and interior volumes, tracks
//! moving characters/parts to stamp snow footprints, and produces the
//! [`WeatherField`] uploaded to the GPU for exterior surface masking and
//! footprint depression.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axcenity_core::{ClassName, Instance, InstanceId, ParticleType, Value, Vector3, World};
use axcenity_render::glam::{Quat, Vec3};
use axcenity_render::{
    DEFAULT_SNOW_THICKNESS, Sprite, TextureSource, WEATHER_FIELD_SIZE, Weather, WeatherField,
};

use crate::orientation::rotation_from_orientation;
use crate::particles::{Random, Surfaces, built_in, global_wind, rain_streak};

/// The most raindrops and snowflakes falling at once, at Intensity 1.
const MOST_DROPS: usize = 2500;
const MOST_FLAKES: usize = 2000;
/// The box around the camera they fall in, in studs. A drop leaving one
/// side comes back in the other, so the camera never runs out of them.
const BOX: Vec3 = Vec3::new(64.0, 48.0, 64.0);
/// How fast rain and snow fall in still air, in studs a second.
const RAIN_SPEED: f32 = 60.0;
const SNOW_SPEED: f32 = 5.0;
/// How far snowflakes sway from side to side as they fall, in studs a
/// second.
const SNOW_SWAY: f32 = 1.5;

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
/// Maximum number of active snow footprints/trails kept at once.
const MAX_FOOTPRINTS: usize = 512;
/// Minimum horizontal travel in studs before stamping the next footprint.
const FOOTPRINT_STEP_DISTANCE: f32 = 1.25;

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
}

impl Eased {
    fn settled(target: &Target) -> Self {
        Self {
            rain: target.rain,
            snow: target.snow,
            wetness: target.wetness,
            cover: target.cover,
            snow_thickness: target.snow_thickness,
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

/// One stamped snow footprint or trail depression in world XZ space.
#[derive(Clone, Copy, Debug)]
struct Footprint {
    x: f32,
    z: f32,
    radius: f32,
    depth: f32,
    age: f32,
}

/// Tracks a moving character or part so footprints stamp at regular step
/// intervals.
#[derive(Clone, Copy, Debug)]
struct MoverTrack {
    last_pos: Vec3,
    distance: f32,
    left_foot: bool,
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
    /// cells so dynamic roofs/doors stay fresh.
    fn update(&mut self, surfaces: &mut Surfaces) {
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
        let hits = surfaces(None, &paths);
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
    /// Last camera position observed by [`add_weather`](crate::Particles::add_weather).
    last_camera: Cell<Option<Vec3>>,
    /// Cached sky-visibility grid around the camera.
    sky_grid: Option<SkyGrid>,
    /// Whether precipitation & surface occlusion is enabled.
    occlusion_enabled: bool,
    /// Cached manual interior/exterior overrides parsed from `Weather.SurfaceMask`.
    manual_mask_raw: String,
    manual_mask: BTreeMap<(i32, i32), SurfaceMaskKind>,
    /// Cached `Interior` `GraphicsVolume` boxes for precipitation occlusion.
    interiors: Vec<InteriorBox>,
    /// Active dynamic snow footprints/trails.
    footprints: Vec<Footprint>,
    /// Movement trackers for characters and dynamic parts.
    movers: HashMap<InstanceId, MoverTrack>,
    /// Cached GPU weather field built during `update`.
    field: Option<Arc<WeatherField>>,
}

impl WeatherState {
    /// Starts again from the world's weather as it is, such as when Play
    /// starts or stops.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Records the camera position for the next sky-visibility update.
    pub(crate) fn note_camera(&self, camera: Vec3) {
        if camera.is_finite() {
            self.last_camera.set(Some(camera));
        }
    }

    /// Eases on by `seconds` toward the world's weather, moves the rain and
    /// snow, updates sky-visibility occlusion via `surfaces`, and stamps snow
    /// footprints for moving characters and parts.
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
        if now.rain > 0.0 {
            fill(&mut self.drops, MOST_DROPS, random);
            for drop in &mut self.drops {
                let velocity = wind - Vec3::Y * RAIN_SPEED * drop.speed;
                drop.position = (drop.position + velocity * seconds).rem_euclid(BOX);
            }
        }
        if now.snow > 0.0 {
            fill(&mut self.flakes, MOST_FLAKES, random);
            self.clock += seconds;
            for flake in &mut self.flakes {
                let sway = self.clock * 0.8 + flake.phase;
                let velocity = wind - Vec3::Y * SNOW_SPEED * flake.speed
                    + Vec3::new(sway.sin(), 0.0, sway.cos()) * SNOW_SWAY;
                flake.position = (flake.position + velocity * seconds).rem_euclid(BOX);
            }
        }

        let weather_active =
            now.rain > 0.0 || now.snow > 0.0 || now.wetness > 0.0 || now.cover > 0.0;
        if !weather_active {
            self.footprints.clear();
            self.movers.clear();
            self.interiors.clear();
            self.field = None;
            return;
        }

        // Collect Interior GraphicsVolume boxes for precipitation occlusion.
        self.interiors = collect_interior_boxes(world);

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
            grid.update(surfaces);
        }

        // Update dynamic snow footprints/trails.
        if target.footprints && now.cover > 0.05 {
            self.update_footprints(world, seconds, now.snow, target.footprint_fade);
        } else if !self.footprints.is_empty() {
            self.footprints.clear();
            self.movers.clear();
        }

        // Build GPU WeatherField when occlusion, manual masks, or footprints are active.
        let needs_field =
            self.occlusion_enabled || !self.manual_mask.is_empty() || !self.footprints.is_empty();
        if needs_field {
            self.field = Some(Arc::new(self.build_field(origin, target.footprint_fade)));
        } else {
            self.field = None;
        }
    }

    fn update_footprints(&mut self, world: &World, seconds: f32, falling_snow: f32, fade: f32) {
        // Falling snow fills footprints faster.
        let age_step = seconds * (1.0 + falling_snow * 1.5);
        for fp in &mut self.footprints {
            fp.age += age_step;
        }
        self.footprints.retain(|fp| fp.age < fade);

        let mut seen = Vec::new();
        for id in world.descendants(world.workspace()) {
            let Some(instance) = world.get(id) else {
                continue;
            };
            match instance.class() {
                ClassName::Model if has_humanoid_child(world, instance) => {
                    if let Some(pos) = character_position(world, instance) {
                        seen.push((id, pos, 0.7, true));
                    }
                }
                ClassName::Part | ClassName::MeshPart => {
                    let anchored = instance
                        .property("Anchored")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    if !anchored
                        && !is_in_character(world, instance)
                        && let Some((pos, size)) = part_position_and_size(instance)
                    {
                        let radius = (size.x.max(size.z) * 0.5).clamp(0.4, 2.5);
                        seen.push((id, pos, radius, false));
                    }
                }
                _ => {}
            }
        }

        self.movers
            .retain(|id, _| seen.iter().any(|(seen_id, _, _, _)| seen_id == id));

        for (id, pos, radius, is_character) in seen {
            let entry = self.movers.entry(id).or_insert(MoverTrack {
                last_pos: pos,
                distance: 0.0,
                left_foot: false,
            });
            let delta = pos - entry.last_pos;
            let horiz = Vec3::new(delta.x, 0.0, delta.z);
            let dist = horiz.length();
            entry.last_pos = pos;
            if !(1e-4..=20.0).contains(&dist) {
                // Ignore stationary objects or teleports.
                continue;
            }
            entry.distance += dist;
            let step = if is_character {
                FOOTPRINT_STEP_DISTANCE
            } else {
                (radius * 0.8).max(0.6)
            };
            if entry.distance >= step {
                entry.distance = 0.0;
                let (fx, fz) = if is_character {
                    let forward = horiz / dist;
                    let right = Vec3::new(-forward.z, 0.0, forward.x);
                    let side = if entry.left_foot { -0.45 } else { 0.45 };
                    entry.left_foot = !entry.left_foot;
                    (pos.x + right.x * side, pos.z + right.z * side)
                } else {
                    (pos.x, pos.z)
                };
                if self.footprints.len() >= MAX_FOOTPRINTS {
                    self.footprints.remove(0);
                }
                self.footprints.push(Footprint {
                    x: fx,
                    z: fz,
                    radius,
                    depth: 1.0,
                    age: 0.0,
                });
            }
        }
    }

    fn build_field(&self, origin: [i32; 2], fade: f32) -> WeatherField {
        let count = FIELD_SIZE * FIELD_SIZE;
        let mut cells = vec![[OPEN_SKY_Y, 0.0, 0.0, 0.0]; count];

        if self.occlusion_enabled
            && let Some(grid) = &self.sky_grid
            && grid.origin == origin
        {
            for (cell, &roof) in cells.iter_mut().zip(grid.roof_y.iter()) {
                cell[0] = roof;
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

        if !self.footprints.is_empty() {
            let inv_fade = 1.0 / fade.max(0.5);
            for fp in &self.footprints {
                let strength = (1.0 - fp.age * inv_fade).clamp(0.0, 1.0) * fp.depth;
                if strength <= 0.001 {
                    continue;
                }
                let r = fp.radius.max(0.6);
                let r2 = r * r;
                let min_gx = ((fp.x - r).floor() as i32 - origin[0]).max(0);
                let max_gx = ((fp.x + r).floor() as i32 - origin[0]).min(FIELD_SIZE as i32 - 1);
                let min_gz = ((fp.z - r).floor() as i32 - origin[1]).max(0);
                let max_gz = ((fp.z + r).floor() as i32 - origin[1]).min(FIELD_SIZE as i32 - 1);
                for gz in min_gz..=max_gz {
                    for gx in min_gx..=max_gx {
                        let wx = (origin[0] + gx) as f32 + 0.5;
                        let wz = (origin[1] + gz) as f32 + 0.5;
                        let dx = wx - fp.x;
                        let dz = wz - fp.z;
                        let d2 = dx * dx + dz * dz;
                        if d2 < r2 {
                            let falloff = 1.0 - d2 / r2;
                            let val = falloff * falloff * strength;
                            let cell = &mut cells[(gz as usize) * FIELD_SIZE + (gx as usize)];
                            if val > cell[2] {
                                cell[2] = val;
                            }
                        }
                    }
                }
            }
        }

        WeatherField {
            origin: [origin[0] as f32, origin[1] as f32],
            cell_size: FIELD_CELL_SIZE,
            size: FIELD_SIZE as u32,
            cells,
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
        })
    }

    /// The latest GPU weather field, if occlusion, manual masks, or footprints
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
                    || !self.footprints.is_empty()
                    || now != Eased::settled(&Target::of(world))
            }
        }
    }

    /// True when a precipitation drop at `position` is occluded by a roof,
    /// painted interior cell, or `Interior` `GraphicsVolume`.
    fn is_drop_occluded(&self, position: Vec3) -> bool {
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
    /// them as how hard each falls now, fading out toward the box's sides,
    /// and culled when occluded by roofs or interior zones.
    pub(crate) fn sprites(&self, camera: Vec3) -> Vec<Sprite> {
        let Some(now) = self.now else {
            return Vec::new();
        };
        let mut sprites = Vec::new();
        let rain = Look {
            size: 1.4,
            color: [0.75, 0.8, 0.9],
            opacity: 0.45,
            texture: rain_streak(),
        };
        let snow = Look {
            size: 0.3,
            color: [1.0; 3],
            opacity: 0.9,
            texture: built_in(ParticleType::Circle),
        };
        for (drops, amount, look) in [
            (&self.drops, now.rain, rain),
            (&self.flakes, now.snow, snow),
        ] {
            let shown = (amount * drops.len() as f32).round() as usize;
            for drop in &drops[..shown.min(drops.len())] {
                let position = around(drop.position, camera);
                if self.is_drop_occluded(position) {
                    continue;
                }
                let out = ((position - camera) / (BOX * 0.5)).length().min(1.0);
                sprites.push(Sprite {
                    position,
                    size: look.size,
                    rotation_degrees: 0.0,
                    color: look.color,
                    opacity: look.opacity * (1.0 - out * out),
                    brightness: 1.0,
                    light_emission: 0.0,
                    texture: Some(look.texture.clone()),
                    frame: None,
                });
            }
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

fn has_humanoid_child(world: &World, instance: &Instance) -> bool {
    instance
        .children()
        .iter()
        .filter_map(|&cid| world.get(cid))
        .any(|child| child.class() == ClassName::Humanoid)
}

fn is_in_character(world: &World, instance: &Instance) -> bool {
    let mut current = instance.parent();
    while let Some(pid) = current {
        let Some(parent) = world.get(pid) else {
            break;
        };
        if parent.class() == ClassName::Model && has_humanoid_child(world, parent) {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn character_position(world: &World, model: &Instance) -> Option<Vec3> {
    for name in ["HumanoidRootPart", "Torso", "UpperTorso", "LowerTorso"] {
        if let Some(child) = model
            .children()
            .iter()
            .filter_map(|&cid| world.get(cid))
            .find(|c| c.name() == name)
            && let Some((pos, _)) = part_position_and_size(child)
        {
            return Some(pos);
        }
    }
    model
        .children()
        .iter()
        .filter_map(|&cid| world.get(cid))
        .find_map(part_position_and_size)
        .map(|(pos, _)| pos)
}

fn part_position_and_size(instance: &Instance) -> Option<(Vec3, Vec3)> {
    let pos = match instance.property("Position") {
        Some(Value::Vector3(p)) => Vec3::new(p.x, p.y, p.z),
        _ => return None,
    };
    let size = match instance.property("Size") {
        Some(Value::Vector3(s)) => Vec3::new(s.x, s.y, s.z),
        _ => Vec3::ONE,
    };
    Some((pos, size))
}

/// How raindrops or snowflakes look.
struct Look {
    size: f32,
    color: [f32; 3],
    opacity: f32,
    texture: Arc<dyn TextureSource>,
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
    use axcenity_core::EnumItem;

    fn no_surfaces(
        _skip: Option<InstanceId>,
        paths: &[(Vector3, Vector3)],
    ) -> Vec<Option<SurfaceHit>> {
        vec![None; paths.len()]
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
        assert!(state.sprites(Vec3::ZERO).is_empty(), "no rain left");

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
        let drops = state.sprites(camera);
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

        // Half as hard, half as many; snow in place of rain once it's eased.
        set(&mut world, weather, "Intensity", Value::Number(0.5));
        set(&mut world, weather, "Precipitation", falling("Snow"));
        set(&mut world, weather, "TransitionTime", Value::Number(0.0));
        state.update(&world, 0.1, &mut random, &mut surfaces);
        assert_eq!(state.sprites(camera).len(), MOST_FLAKES / 2);
    }

    #[test]
    fn roof_occludes_rain_drops_below_it_and_populates_weather_field() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Intensity", Value::Number(1.0));
        let mut random = Random::default();
        let mut state = WeatherState::default();
        let camera = Vec3::new(0.0, 5.0, 0.0);
        state.note_camera(camera);

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
        let drops = state.sprites(camera);
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

    #[test]
    fn moving_character_stamps_snow_footprints_that_fade_over_time() {
        let (mut world, weather) = world_with_weather();
        set(&mut world, weather, "Precipitation", falling("None"));
        set(&mut world, weather, "SnowCover", Value::Number(1.0));
        set(&mut world, weather, "FootprintFadeTime", Value::Number(5.0));

        let model = world.create(ClassName::Model).unwrap();
        world.set_parent(model, Some(world.workspace())).unwrap();
        let hum = world.create(ClassName::Humanoid).unwrap();
        world.set_parent(hum, Some(model)).unwrap();
        let root = world.create(ClassName::Part).unwrap();
        world.set_name(root, "HumanoidRootPart").unwrap();
        set(
            &mut world,
            root,
            "Position",
            Value::Vector3(Vector3::new(0.0, 0.0, 0.0)),
        );
        world.set_parent(root, Some(model)).unwrap();

        let mut random = Random::default();
        let mut state = WeatherState::default();
        let mut surfaces = no_surfaces;
        state.note_camera(Vec3::ZERO);
        state.update(&world, 0.1, &mut random, &mut surfaces);
        assert!(state.footprints.is_empty());

        // Move character forward by 2 studs -> stamps a footprint.
        set(
            &mut world,
            root,
            "Position",
            Value::Vector3(Vector3::new(0.0, 0.0, 2.0)),
        );
        state.update(&world, 0.1, &mut random, &mut surfaces);
        assert_eq!(state.footprints.len(), 1);
        let field = state.field().expect("field with footprint");
        assert!(
            field.cells.iter().any(|c| c[2] > 0.1),
            "footprint baked into weather_field channel z"
        );

        // Advance past FootprintFadeTime -> footprint fades away.
        for _ in 0..30 {
            state.update(&world, 0.2, &mut random, &mut surfaces);
        }
        assert!(state.footprints.is_empty(), "footprints fade back out");
    }
}
