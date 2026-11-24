//! Environment presets: the whole look of the sky and the weather under one
//! name, blended from one to the next.
//!
//! A preset ([`EnvironmentPreset`]) holds values of Lighting, its first
//! Atmosphere, Clouds, Weather and ColorCorrectionEffect, and the
//! Workspace's `GlobalWind`. A value it leaves out is one it doesn't touch.
//! Six are built in ([`BUILT_IN`]: Clear, Evening, Overcast, Storm, SnowDay
//! and SnowNight); a project's own are files,
//! `assets/environments/<name>.environment.json`, and one named like a
//! built-in replaces it ([`find_preset`]).
//!
//! - [`blend`] mixes two presets, purely: numbers, colors and vectors go
//!   straight from one to the other, switches and choices change half way,
//!   `ClockTime` turns round the clock ([`ClockTurn`]) and rain never
//!   becomes snow without stopping first.
//! - [`apply_preset`] puts a preset's values into a World, and
//!   [`capture`] reads a World's into a preset.
//! - [`EnvironmentTrack`] is a sequence's Environment track: keys that name
//!   presets at times ([`EnvironmentKey`]), sampled at any time.
//! - [`EnvironmentTransition`] eases a World from how it is to a preset
//!   over some seconds (`Lighting:TransitionToPreset` in scripts).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use axcenity_assets::Assets;
use axcenity_core::{ClassName, Color3, EnumItem, InstanceId, Value, Vector3, World};
use serde::{Deserialize, Serialize};

use crate::easing::{DEFAULT_BEZIER, Easing, EasingDirection, EasingStyle, cubic_bezier};
use crate::files::{FileError, Interp, keyed_choices};

pub const ENVIRONMENTS_FOLDER: &str = "assets/environments";
pub const ENVIRONMENT_EXTENSION: &str = ".environment.json";
pub const ENVIRONMENT_FORMAT: u32 = 1;

/// The names of the built-in presets, in the order pickers list them.
pub const BUILT_IN: [&str; 6] = [
    "Clear",
    "Evening",
    "Overcast",
    "Storm",
    "SnowDay",
    "SnowNight",
];

/// The path of the project's preset called `name`.
pub fn preset_path(name: &str) -> String {
    format!(
        "{ENVIRONMENTS_FOLDER}/{}{ENVIRONMENT_EXTENSION}",
        name.trim()
    )
}

/// The name of the preset at `path` (`Dusk` for
/// `assets/environments/Dusk.environment.json`), or `None` for another
/// kind of file.
pub fn preset_name(path: &str) -> Option<&str> {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(ENVIRONMENT_EXTENSION)
        .filter(|name| !name.is_empty())
}

/// What a preset's values belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvironmentTarget {
    /// The Lighting service.
    Lighting,
    /// Lighting's first Atmosphere.
    Atmosphere,
    /// Lighting's first Clouds.
    Clouds,
    /// Lighting's first Weather.
    Weather,
    /// Lighting's first ColorCorrectionEffect.
    ColorCorrection,
    /// The Workspace (its `GlobalWind`).
    Workspace,
}

impl EnvironmentTarget {
    pub const ALL: [EnvironmentTarget; 6] = [
        EnvironmentTarget::Lighting,
        EnvironmentTarget::Atmosphere,
        EnvironmentTarget::Clouds,
        EnvironmentTarget::Weather,
        EnvironmentTarget::ColorCorrection,
        EnvironmentTarget::Workspace,
    ];

    pub fn class(self) -> ClassName {
        match self {
            EnvironmentTarget::Lighting => ClassName::Lighting,
            EnvironmentTarget::Atmosphere => ClassName::Atmosphere,
            EnvironmentTarget::Clouds => ClassName::Clouds,
            EnvironmentTarget::Weather => ClassName::Weather,
            EnvironmentTarget::ColorCorrection => ClassName::ColorCorrectionEffect,
            EnvironmentTarget::Workspace => ClassName::Workspace,
        }
    }

    /// Its name in a preset file, which is its class's.
    pub fn name(self) -> &'static str {
        match self {
            EnvironmentTarget::Lighting => "Lighting",
            EnvironmentTarget::Atmosphere => "Atmosphere",
            EnvironmentTarget::Clouds => "Clouds",
            EnvironmentTarget::Weather => "Weather",
            EnvironmentTarget::ColorCorrection => "ColorCorrectionEffect",
            EnvironmentTarget::Workspace => "Workspace",
        }
    }

    /// The Instance in `world` its values go to, or `None` when the world
    /// has none (no Clouds in Lighting, say).
    pub fn find(self, world: &World) -> Option<InstanceId> {
        match self {
            EnvironmentTarget::Lighting => Some(world.lighting()),
            EnvironmentTarget::Workspace => Some(world.workspace()),
            _ => {
                let class = self.class();
                world
                    .get(world.lighting())?
                    .children()
                    .iter()
                    .copied()
                    .find(|&id| world.get(id).is_some_and(|child| child.class() == class))
            }
        }
    }
}

/// One value of a preset: a number, a switch, a color or vector (three
/// numbers), or a choice by its item's name (`"Rain"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PresetValue {
    Number(f64),
    Switch(bool),
    Triple([f32; 3]),
    Choice(String),
}

impl PresetValue {
    /// `value` as a preset holds it, or `None` for a kind it can't hold
    /// (text, most enums).
    pub fn of(value: &Value) -> Option<PresetValue> {
        Some(match value {
            Value::Number(n) => PresetValue::Number(*n),
            Value::Bool(on) => PresetValue::Switch(*on),
            Value::Vector3(v) => PresetValue::Triple([v.x, v.y, v.z]),
            Value::Color3(c) => PresetValue::Triple([c.r, c.g, c.b]),
            Value::EnumItem(item) if keyed_choices(&item.enum_name).is_some() => {
                PresetValue::Choice(item.item.clone())
            }
            _ => return None,
        })
    }

    /// This as a property value of the same kind as `like`, or `None` if
    /// it doesn't fit.
    pub fn to_value(&self, like: &Value) -> Option<Value> {
        Some(match (self, like) {
            (PresetValue::Number(n), Value::Number(_)) if n.is_finite() => Value::Number(*n),
            (PresetValue::Switch(on), Value::Bool(_)) => Value::Bool(*on),
            (PresetValue::Triple([x, y, z]), Value::Vector3(_)) => {
                Value::Vector3(Vector3::new(*x, *y, *z))
            }
            (PresetValue::Triple([r, g, b]), Value::Color3(_)) => {
                Value::Color3(Color3::new(*r, *g, *b))
            }
            (PresetValue::Choice(item), Value::EnumItem(like)) => {
                let items = keyed_choices(&like.enum_name)?;
                items.contains(&item.as_str()).then(|| {
                    Value::EnumItem(EnumItem {
                        enum_name: like.enum_name.clone(),
                        item: item.clone(),
                    })
                })?
            }
            _ => return None,
        })
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            PresetValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_choice(&self) -> Option<&str> {
        match self {
            PresetValue::Choice(item) => Some(item),
            _ => None,
        }
    }
}

/// The values of one [`EnvironmentTarget`], by property name.
pub type PresetValues = BTreeMap<String, PresetValue>;

/// A named look of the sky and the weather. A property it leaves out is
/// one it doesn't touch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentPreset {
    pub format_version: u32,
    #[serde(
        rename = "Lighting",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub lighting: PresetValues,
    #[serde(
        rename = "Atmosphere",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub atmosphere: PresetValues,
    #[serde(rename = "Clouds", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub clouds: PresetValues,
    #[serde(
        rename = "Weather",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub weather: PresetValues,
    #[serde(
        rename = "ColorCorrectionEffect",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub color_correction: PresetValues,
    #[serde(
        rename = "Workspace",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub workspace: PresetValues,
}

impl Default for EnvironmentPreset {
    fn default() -> Self {
        Self {
            format_version: ENVIRONMENT_FORMAT,
            lighting: PresetValues::new(),
            atmosphere: PresetValues::new(),
            clouds: PresetValues::new(),
            weather: PresetValues::new(),
            color_correction: PresetValues::new(),
            workspace: PresetValues::new(),
        }
    }
}

/// What each property a preset can hold is like: the default of every
/// number, Vector3, Color3, switch and keyed choice of the targets'
/// classes, and of the Workspace only `GlobalWind`.
fn property_table() -> &'static BTreeMap<EnvironmentTarget, BTreeMap<&'static str, Value>> {
    static TABLE: OnceLock<BTreeMap<EnvironmentTarget, BTreeMap<&'static str, Value>>> =
        OnceLock::new();
    TABLE.get_or_init(|| {
        EnvironmentTarget::ALL
            .into_iter()
            .map(|target| {
                let wind = target == EnvironmentTarget::Workspace;
                let properties = target
                    .class()
                    .default_properties()
                    .into_iter()
                    .filter(|(name, value)| {
                        PresetValue::of(value).is_some() && (!wind || *name == "GlobalWind")
                    })
                    .collect();
                (target, properties)
            })
            .collect()
    })
}

/// What `name` of `target` is like, if a preset can hold it.
fn like(target: EnvironmentTarget, name: &str) -> Option<&'static Value> {
    property_table().get(&target)?.get(name)
}

/// The properties [`capture`] reads: the look, not the settings around it
/// (shadows, the Weather's `TransitionTime` and masks stay the world's).
pub const CAPTURED: [(EnvironmentTarget, &[&str]); 6] = [
    (
        EnvironmentTarget::Lighting,
        &["ClockTime", "Exposure", "EnvironmentIntensity"],
    ),
    (
        EnvironmentTarget::Atmosphere,
        &["Density", "Haze", "Glare", "Color", "Decay"],
    ),
    (
        EnvironmentTarget::Clouds,
        &["Cover", "Density", "Color", "Speed", "Direction"],
    ),
    (
        EnvironmentTarget::Weather,
        &[
            "Precipitation",
            "Intensity",
            "Wetness",
            "SnowCover",
            "SnowThickness",
        ],
    ),
    (
        EnvironmentTarget::ColorCorrection,
        &["Saturation", "Brightness", "Contrast", "TintColor"],
    ),
    (EnvironmentTarget::Workspace, &["GlobalWind"]),
];

impl EnvironmentPreset {
    /// A preset that touches nothing.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn values(&self, target: EnvironmentTarget) -> &PresetValues {
        match target {
            EnvironmentTarget::Lighting => &self.lighting,
            EnvironmentTarget::Atmosphere => &self.atmosphere,
            EnvironmentTarget::Clouds => &self.clouds,
            EnvironmentTarget::Weather => &self.weather,
            EnvironmentTarget::ColorCorrection => &self.color_correction,
            EnvironmentTarget::Workspace => &self.workspace,
        }
    }

    pub fn values_mut(&mut self, target: EnvironmentTarget) -> &mut PresetValues {
        match target {
            EnvironmentTarget::Lighting => &mut self.lighting,
            EnvironmentTarget::Atmosphere => &mut self.atmosphere,
            EnvironmentTarget::Clouds => &mut self.clouds,
            EnvironmentTarget::Weather => &mut self.weather,
            EnvironmentTarget::ColorCorrection => &mut self.color_correction,
            EnvironmentTarget::Workspace => &mut self.workspace,
        }
    }

    pub fn get(&self, target: EnvironmentTarget, name: &str) -> Option<&PresetValue> {
        self.values(target).get(name)
    }

    /// Sets `name` of `target`, replacing what it held.
    pub fn set(&mut self, target: EnvironmentTarget, name: &str, value: PresetValue) {
        self.values_mut(target).insert(name.to_owned(), value);
    }

    /// `name` of `target` as a number, if the preset sets one.
    pub fn number(&self, target: EnvironmentTarget, name: &str) -> Option<f64> {
        self.get(target, name).and_then(PresetValue::as_number)
    }

    /// Whether it sets nothing.
    pub fn is_empty(&self) -> bool {
        EnvironmentTarget::ALL
            .iter()
            .all(|&target| self.values(target).is_empty())
    }

    /// Every property it sets.
    pub fn fields(&self) -> impl Iterator<Item = (EnvironmentTarget, &str, &PresetValue)> {
        EnvironmentTarget::ALL.into_iter().flat_map(move |target| {
            self.values(target)
                .iter()
                .map(move |(name, value)| (target, name.as_str(), value))
        })
    }

    /// Checks every value is one its target has, of the right kind.
    pub fn check(&self) -> Result<(), FileError> {
        for (target, name, value) in self.fields() {
            let Some(like) = like(target, name) else {
                return Err(FileError(format!(
                    "{}.{name} isn't a number, color, vector, switch or choice a preset can hold",
                    target.name()
                )));
            };
            if value.to_value(like).is_none() {
                return Err(FileError(format!(
                    "{}.{name} is {}; it takes {}",
                    target.name(),
                    described(value),
                    match like {
                        Value::Number(_) => "a number".to_owned(),
                        Value::Bool(_) => "true or false".to_owned(),
                        Value::EnumItem(item) => format!(
                            "one of {}",
                            keyed_choices(&item.enum_name)
                                .unwrap_or_default()
                                .join(", ")
                        ),
                        _ => "three numbers".to_owned(),
                    }
                )));
            }
        }
        Ok(())
    }

    /// Reads and checks a preset file.
    pub fn from_json(text: &str) -> Result<Self, FileError> {
        let preset: EnvironmentPreset =
            serde_json::from_str(text).map_err(|err| FileError(err.to_string()))?;
        if preset.format_version != ENVIRONMENT_FORMAT {
            return Err(FileError(format!(
                "it's format {}; this engine reads format {ENVIRONMENT_FORMAT}",
                preset.format_version
            )));
        }
        preset.check()?;
        Ok(preset)
    }

    /// Readable JSON, the same for the same preset.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("presets always serialize");
        text.push('\n');
        text
    }
}

fn described(value: &PresetValue) -> String {
    match value {
        PresetValue::Number(n) => format!("the number {n}"),
        PresetValue::Switch(on) => format!("{on}"),
        PresetValue::Triple(_) => "three numbers".to_owned(),
        PresetValue::Choice(item) => format!("\"{item}\""),
    }
}

/// The built-in preset called `name` (any case), if there is one.
pub fn built_in(name: &str) -> Option<EnvironmentPreset> {
    use EnvironmentTarget::{Atmosphere, Clouds, ColorCorrection, Lighting, Weather, Workspace};
    let name = BUILT_IN
        .into_iter()
        .find(|built_in| built_in.eq_ignore_ascii_case(name.trim()))?;
    // Hours, exposure; the air's density and haze; the clouds' cover,
    // density, color and speed; what falls and how hard; wetness and snow
    // cover; the wind; saturation.
    // The cloud's own darkness does the dimming under an overcast: a
    // thick layer at a Density near 1 with a lowered exposure as well came
    // out as dusk at three in the afternoon.
    struct Look {
        clock: f64,
        exposure: f64,
        air: (f64, f64),
        clouds: (f64, f64, [f32; 3], f64),
        falling: (&'static str, f64),
        wetness: f64,
        snow_cover: f64,
        wind: [f32; 3],
        saturation: f64,
    }
    const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
    const SNOW_CLOUD: [f32; 3] = [0.8, 0.82, 0.88];
    let look = match name {
        "Clear" => Look {
            clock: 13.4,
            exposure: 1.0,
            air: (0.395, 0.0),
            clouds: (0.3, 0.7, WHITE, 20.0),
            falling: ("None", 0.0),
            wetness: 0.0,
            snow_cover: 0.0,
            wind: [4.0, 0.0, 1.5],
            saturation: 0.0,
        },
        "Evening" => Look {
            clock: 19.2,
            exposure: 1.0,
            air: (0.395, 0.5),
            clouds: (0.3, 0.7, WHITE, 20.0),
            falling: ("None", 0.0),
            wetness: 0.0,
            snow_cover: 0.0,
            wind: [3.0, 0.0, 1.0],
            saturation: 0.0,
        },
        "Overcast" => Look {
            clock: 13.4,
            exposure: 1.0,
            air: (0.5, 0.6),
            clouds: (0.8, 0.6, [0.72, 0.74, 0.78], 25.0),
            falling: ("None", 0.0),
            wetness: 0.0,
            snow_cover: 0.0,
            wind: [8.0, 0.0, 3.0],
            saturation: -0.15,
        },
        "Storm" => Look {
            clock: 15.0,
            exposure: 1.0,
            air: (0.6, 1.0),
            clouds: (0.85, 0.55, [0.5, 0.52, 0.58], 45.0),
            falling: ("Rain", 0.9),
            wetness: 0.9,
            snow_cover: 0.0,
            wind: [16.0, 0.0, 6.0],
            saturation: -0.25,
        },
        "SnowDay" => Look {
            clock: 13.0,
            exposure: 1.0,
            air: (0.45, 0.4),
            clouds: (0.8, 0.5, SNOW_CLOUD, 15.0),
            falling: ("Snow", 0.6),
            wetness: 0.0,
            snow_cover: 1.0,
            wind: [4.0, 0.0, 1.5],
            saturation: -0.15,
        },
        _ => Look {
            clock: 19.3,
            exposure: 1.0,
            air: (0.45, 0.4),
            clouds: (0.8, 0.5, SNOW_CLOUD, 15.0),
            falling: ("Snow", 0.6),
            wetness: 0.0,
            snow_cover: 1.0,
            wind: [4.0, 0.0, 1.5],
            saturation: -0.15,
        },
    };
    let number = PresetValue::Number;
    let mut preset = EnvironmentPreset::new();
    preset.set(Lighting, "ClockTime", number(look.clock));
    preset.set(Lighting, "Exposure", number(look.exposure));
    preset.set(Atmosphere, "Density", number(look.air.0));
    preset.set(Atmosphere, "Haze", number(look.air.1));
    preset.set(Clouds, "Cover", number(look.clouds.0));
    preset.set(Clouds, "Density", number(look.clouds.1));
    preset.set(Clouds, "Color", PresetValue::Triple(look.clouds.2));
    preset.set(Clouds, "Speed", number(look.clouds.3));
    preset.set(
        Weather,
        "Precipitation",
        PresetValue::Choice(look.falling.0.to_owned()),
    );
    preset.set(Weather, "Intensity", number(look.falling.1));
    preset.set(Weather, "Wetness", number(look.wetness));
    preset.set(Weather, "SnowCover", number(look.snow_cover));
    preset.set(Weather, "SnowThickness", number(0.35));
    preset.set(ColorCorrection, "Saturation", number(look.saturation));
    preset.set(Workspace, "GlobalWind", PresetValue::Triple(look.wind));
    Some(preset)
}

/// Whether `name` can name a preset: not empty, and no path in it.
pub fn is_preset_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && !name.contains(['/', '\\', ':', '\0']) && !name.starts_with('.')
}

/// The preset called `name`: the project's file
/// (`assets/environments/<name>.environment.json`) if it has one, else the
/// built-in. A file that can't be read as a preset is an error, so a
/// broken one isn't quietly replaced by a built-in.
pub fn find_preset(assets: &Assets, name: &str) -> Result<EnvironmentPreset, String> {
    let name = name.trim();
    if !is_preset_name(name) {
        return Err(format!("`{name}` isn't a preset's name"));
    }
    let path = preset_path(name);
    match assets.read_file(&path) {
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| format!("{path} isn't text"))?;
            EnvironmentPreset::from_json(&text).map_err(|err| format!("can't read {path}: {}", err.0))
        }
        Err(_) => built_in(name).ok_or_else(|| {
            format!(
                "there's no environment preset named `{name}`: it isn't built in ({}) and {path} isn't there",
                BUILT_IN.join(", ")
            )
        }),
    }
}

/// The names of the project's own presets (the files in
/// `assets/environments/`), sorted.
pub fn project_presets(assets: &Assets) -> Vec<String> {
    let Some(folder) = assets.folder() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(folder.join(ENVIRONMENTS_FOLDER)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name();
            preset_name(name.to_str()?).map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

/// Every preset's name: the built-ins, then the project's own that aren't
/// named like one.
pub fn preset_names(assets: &Assets) -> Vec<String> {
    let mut names: Vec<String> = BUILT_IN.iter().map(|name| (*name).to_owned()).collect();
    for name in project_presets(assets) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Which way `ClockTime` turns from one preset to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockTurn {
    /// The short way round: 22 to 2 goes through midnight, and 6 to 20
    /// goes back through midnight too (10 hours, not 14).
    #[default]
    Shortest,
    /// As time passes: 6 to 20 goes through noon.
    Forward,
    /// Against time.
    Backward,
}

impl ClockTurn {
    pub const ALL: [ClockTurn; 3] = [ClockTurn::Shortest, ClockTurn::Forward, ClockTurn::Backward];

    pub fn is_shortest(&self) -> bool {
        *self == ClockTurn::Shortest
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ClockTurn::Shortest => "Shortest",
            ClockTurn::Forward => "Forward",
            ClockTurn::Backward => "Backward",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|turn| turn.as_str() == name)
    }
}

/// The hour `t` of the way from `a` to `b` round a 24 hour clock.
pub fn blend_clock(a: f64, b: f64, t: f64, turn: ClockTurn) -> f64 {
    let forward = (b - a).rem_euclid(24.0);
    let hours = match turn {
        ClockTurn::Shortest if forward > 12.0 => forward - 24.0,
        ClockTurn::Backward if forward > 0.0 => forward - 24.0,
        _ => forward,
    };
    (a + hours * t).rem_euclid(24.0)
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// `t` of the way from `a` to `b`: numbers and triples go straight, and
/// anything with nothing between (a switch, a choice, two kinds) changes
/// half way.
fn blend_value(a: &PresetValue, b: &PresetValue, t: f64) -> PresetValue {
    match (a, b) {
        (PresetValue::Number(a), PresetValue::Number(b)) => PresetValue::Number(mix(*a, *b, t)),
        (PresetValue::Triple(a), PresetValue::Triple(b)) => {
            let t = t as f32;
            PresetValue::Triple(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t))
        }
        _ if t < 0.5 => a.clone(),
        _ => b.clone(),
    }
}

/// What falls and how hard, `t` of the way from one weather to another.
/// Nothing falling counts as an intensity of 0 of whatever the other has,
/// so rain starts and stops gently instead of switching on half way; rain
/// turning to snow (or back) eases to nothing by half way, changes there,
/// and eases in again, so the two never fall together.
fn blend_falling(a: (&str, f64), b: (&str, f64), t: f64) -> (String, f64) {
    let none = |kind: &str| kind == "None";
    let hard = |(kind, intensity): (&str, f64)| if none(kind) { 0.0 } else { intensity };
    if a.0 == b.0 {
        return (a.0.to_owned(), mix(a.1, b.1, t));
    }
    if none(a.0) {
        return (b.0.to_owned(), mix(0.0, hard(b), t));
    }
    if none(b.0) {
        return (a.0.to_owned(), mix(hard(a), 0.0, t));
    }
    if t < 0.5 {
        (a.0.to_owned(), a.1 * (1.0 - 2.0 * t))
    } else {
        (b.0.to_owned(), b.1 * (2.0 * t - 1.0))
    }
}

/// `t` (0 to 1) of the way from `a` to `b`. At 0 it's `a` and at 1 `b`,
/// exactly. A value only one of them sets is kept as that one has it; a
/// value neither sets isn't in the result.
pub fn blend(
    a: &EnvironmentPreset,
    b: &EnvironmentPreset,
    t: f64,
    turn: ClockTurn,
) -> EnvironmentPreset {
    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    let mut out = a.clone();
    for (target, name, value) in b.fields() {
        let blended = match a.get(target, name) {
            Some(_) if t >= 1.0 => value.clone(),
            Some(from) if t <= 0.0 => from.clone(),
            Some(from) => blend_value(from, value, t),
            None => value.clone(),
        };
        out.set(target, name, blended);
    }
    if !(0.0 < t && t < 1.0) {
        return out;
    }
    let lighting = EnvironmentTarget::Lighting;
    if let (Some(from), Some(to)) = (
        a.number(lighting, "ClockTime"),
        b.number(lighting, "ClockTime"),
    ) {
        out.set(
            lighting,
            "ClockTime",
            PresetValue::Number(blend_clock(from, to, t, turn)),
        );
    }
    let weather = EnvironmentTarget::Weather;
    let falling = |preset: &'_ EnvironmentPreset| {
        let kind = preset
            .get(weather, "Precipitation")
            .and_then(PresetValue::as_choice)
            .map(str::to_owned)?;
        Some((kind, preset.number(weather, "Intensity")?))
    };
    if let (Some(from), Some(to)) = (falling(a), falling(b)) {
        let (kind, intensity) = blend_falling((&from.0, from.1), (&to.0, to.1), t);
        out.set(weather, "Precipitation", PresetValue::Choice(kind));
        out.set(weather, "Intensity", PresetValue::Number(intensity));
    }
    out
}

/// Puts `preset`'s values into `world`, leaving out the ones `skip` says
/// something else sets (by Instance and property name). A value the world
/// already has isn't set again. Returns the targets the world hasn't got
/// (a preset with clouds in a world with no Clouds), whose values went
/// nowhere.
pub fn apply_preset(
    world: &mut World,
    preset: &EnvironmentPreset,
    skip: impl Fn(InstanceId, &str) -> bool,
) -> Vec<EnvironmentTarget> {
    let mut missing = Vec::new();
    for target in EnvironmentTarget::ALL {
        let values = preset.values(target);
        if values.is_empty() {
            continue;
        }
        let Some(id) = target.find(world) else {
            missing.push(target);
            continue;
        };
        for (name, value) in values {
            if skip(id, name) {
                continue;
            }
            let now = world.get(id).and_then(|instance| instance.property(name));
            let Some(value) = now.and_then(|like| value.to_value(like)) else {
                continue;
            };
            if now != Some(&value) {
                let _ = world.set_property(id, name, value);
            }
        }
    }
    missing
}

/// `world`'s values of `names` of each target, as a preset. A target the
/// world hasn't got is left out.
fn read(
    world: &World,
    wanted: impl Iterator<Item = (EnvironmentTarget, String)>,
) -> EnvironmentPreset {
    let mut preset = EnvironmentPreset::new();
    for (target, name) in wanted {
        let value = target
            .find(world)
            .and_then(|id| world.get(id))
            .and_then(|instance| instance.property(&name))
            .and_then(PresetValue::of);
        if let Some(value) = value {
            preset.set(target, &name, value);
        }
    }
    preset
}

/// The world's look as a preset: the properties in [`CAPTURED`] of
/// Lighting, of the Atmosphere, Clouds, Weather and ColorCorrectionEffect
/// it has, and the wind.
pub fn capture(world: &World) -> EnvironmentPreset {
    read(
        world,
        CAPTURED.iter().flat_map(|(target, names)| {
            names.iter().map(move |name| (*target, (*name).to_owned()))
        }),
    )
}

/// The world's own values of what `like` sets: where a blend toward `like`
/// starts from.
pub fn capture_like(world: &World, like: &EnvironmentPreset) -> EnvironmentPreset {
    read(
        world,
        like.fields()
            .map(|(target, name, _)| (target, name.to_owned())),
    )
}

/// One key of a sequence's Environment track: at `time` the world looks
/// like `preset`, and gets to the next key's as `interp` says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentKey {
    pub id: String,
    pub time: f64,
    /// A preset's name: a built-in or one of the project's.
    pub preset: String,
    #[serde(default)]
    pub interp: Interp,
    /// An `Ease` key's style and direction (Quad InOut if left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easing: Option<Easing>,
    /// A `Bezier` key's handles, as a property key's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bezier: Option<[f32; 4]>,
    /// Which way the clock turns to the next key. Left out for the short
    /// way.
    #[serde(default, skip_serializing_if = "ClockTurn::is_shortest")]
    pub clock: ClockTurn,
}

impl EnvironmentKey {
    /// A key that eases gently to the next.
    pub fn new(id: impl Into<String>, time: f64, preset: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            time,
            preset: preset.into(),
            interp: Interp::Ease,
            easing: Some(Easing::new(EasingStyle::Sine, EasingDirection::InOut)),
            bezier: None,
            clock: ClockTurn::Shortest,
        }
    }

    /// How far along the blend to the next key is at `t`, 0 to 1 of the
    /// way there in time, as a property key's.
    pub fn progress(&self, t: f32) -> f32 {
        match self.interp {
            Interp::Step => 0.0,
            Interp::Linear => t.clamp(0.0, 1.0),
            Interp::Ease => self.easing.unwrap_or_default().apply(t),
            Interp::Bezier => cubic_bezier(self.bezier.unwrap_or(DEFAULT_BEZIER), t),
        }
    }
}

/// A key with its preset found.
#[derive(Clone, Debug, PartialEq)]
struct Keyed {
    key: EnvironmentKey,
    /// Its preset, with every value another key sets and it doesn't
    /// filled in from the keys around it.
    preset: EnvironmentPreset,
    /// Which way the clock turns from here: its own, or (when its preset
    /// has no `ClockTime`) the turn it's part of.
    clock: ClockTurn,
}

/// A sequence's Environment track, ready to sample.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnvironmentTrack {
    keys: Vec<Keyed>,
    /// The presets its keys name that weren't found. Those keys are left
    /// out.
    pub missing: BTreeSet<String>,
}

impl EnvironmentTrack {
    /// The track of `keys`, each preset found by `find`.
    ///
    /// A value is keyed only where a preset sets it: a key whose preset
    /// leaves a value out passes through what the keys around it that do
    /// set it blend to at its time, so a preset of only the wind between
    /// two whole ones leaves the sky blending straight across.
    pub fn new(
        keys: &[EnvironmentKey],
        mut find: impl FnMut(&str) -> Option<EnvironmentPreset>,
    ) -> Self {
        let mut missing = BTreeSet::new();
        let mut found: Vec<(EnvironmentKey, EnvironmentPreset)> = keys
            .iter()
            .filter_map(|key| match find(&key.preset) {
                Some(preset) => Some((key.clone(), preset)),
                None => {
                    missing.insert(key.preset.clone());
                    None
                }
            })
            .collect();
        found.sort_by(|a, b| a.0.time.total_cmp(&b.0.time));
        let fields: BTreeSet<(EnvironmentTarget, String)> = found
            .iter()
            .flat_map(|(_, preset)| preset.fields())
            .map(|(target, name, _)| (target, name.to_owned()))
            .collect();
        let sets = |index: usize, target, name: &str| found[index].1.get(target, name).is_some();
        let mut keyed: Vec<Keyed> = found
            .iter()
            .map(|(key, preset)| Keyed {
                key: key.clone(),
                preset: preset.clone(),
                clock: key.clock,
            })
            .collect();
        for index in 0..found.len() {
            for (target, name) in &fields {
                if sets(index, *target, name) {
                    continue;
                }
                let before = (0..index).rev().find(|&i| sets(i, *target, name));
                let after = (index + 1..found.len()).find(|&i| sets(i, *target, name));
                let filled = match (before, after) {
                    (Some(before), Some(after)) => {
                        let (from, to) = (&found[before], &found[after]);
                        let span = to.0.time - from.0.time;
                        let along = if span > 0.0 {
                            (found[index].0.time - from.0.time) / span
                        } else {
                            1.0
                        };
                        let t = f64::from(from.0.progress(along as f32));
                        blend(&from.1, &to.1, t, from.0.clock)
                            .get(*target, name)
                            .cloned()
                    }
                    (Some(held), None) | (None, Some(held)) => {
                        found[held].1.get(*target, name).cloned()
                    }
                    (None, None) => None,
                };
                if let Some(value) = filled {
                    keyed[index].preset.set(*target, name, value);
                }
                if (*target, name.as_str()) == (EnvironmentTarget::Lighting, "ClockTime")
                    && let Some(before) = before
                {
                    keyed[index].clock = found[before].0.clock;
                }
            }
        }
        Self {
            keys: keyed,
            missing,
        }
    }

    /// The track of `keys` with the built-in presets and the project's.
    pub fn load(keys: &[EnvironmentKey], assets: &Assets) -> Self {
        Self::new(keys, |name| find_preset(assets, name).ok())
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The look at `time` seconds: the first key's before it, the last
    /// key's after it, and between two keys their blend, as far along as
    /// the earlier key's easing says. `None` for a track with no keys.
    pub fn sample(&self, time: f64) -> Option<EnvironmentPreset> {
        let first = self.keys.first()?;
        if time <= first.key.time {
            return Some(first.preset.clone());
        }
        let index = self.keys.iter().rposition(|k| k.key.time <= time)?;
        let from = &self.keys[index];
        let Some(to) = self.keys.get(index + 1) else {
            return Some(from.preset.clone());
        };
        if from.key.interp == Interp::Step {
            return Some(from.preset.clone());
        }
        let span = (to.key.time - from.key.time).max(1e-12);
        let along = ((time - from.key.time) / span) as f32;
        Some(blend(
            &from.preset,
            &to.preset,
            f64::from(from.key.progress(along)),
            from.clock,
        ))
    }
}

/// A world easing from how it was to a preset over some seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentTransition {
    from: EnvironmentPreset,
    to: EnvironmentPreset,
    seconds: f64,
    elapsed: f64,
    turn: ClockTurn,
    /// The targets `to` has values for that the world didn't have when it
    /// started.
    pub missing: Vec<EnvironmentTarget>,
}

impl EnvironmentTransition {
    /// Starts from `world` as it is now toward `to`, over `seconds` (0 or
    /// less gets there at the first step).
    pub fn start(world: &World, to: EnvironmentPreset, seconds: f64, turn: ClockTurn) -> Self {
        let missing = EnvironmentTarget::ALL
            .into_iter()
            .filter(|&target| !to.values(target).is_empty() && target.find(world).is_none())
            .collect();
        Self {
            from: capture_like(world, &to),
            to,
            seconds: if seconds.is_finite() {
                seconds.max(0.0)
            } else {
                0.0
            },
            elapsed: 0.0,
            turn,
            missing,
        }
    }

    /// How far along it is, from 0 to 1 in time.
    pub fn progress(&self) -> f64 {
        if self.seconds > 0.0 {
            (self.elapsed / self.seconds).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// The look now: slow at both ends (Sine InOut).
    pub fn look(&self) -> EnvironmentPreset {
        let eased =
            Easing::new(EasingStyle::Sine, EasingDirection::InOut).apply(self.progress() as f32);
        blend(&self.from, &self.to, f64::from(eased), self.turn)
    }

    /// Moves on by `seconds` and puts the look into `world`. Returns
    /// whether it has arrived.
    pub fn step(&mut self, world: &mut World, seconds: f64) -> bool {
        if seconds.is_finite() {
            self.elapsed += seconds.max(0.0);
        }
        let done = self.progress() >= 1.0;
        let look = if done { self.to.clone() } else { self.look() };
        apply_preset(world, &look, |_, _| false);
        done
    }
}

#[cfg(test)]
mod tests;
