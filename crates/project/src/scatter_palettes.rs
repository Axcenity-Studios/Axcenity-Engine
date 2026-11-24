//! Scatter palettes: `.axscatter` files in the project's `presets/` folder
//! (beside world presets), each a list of what the editor's Scatter pane
//! places, with each item's weight, size, turn, collide, lean and sink, and
//! the palette's density and spacing (`docs/editor/scatter.md`).
//!
//! An item names its plant, prop or model as a ScatterRule's `Models` entry
//! does (`prop:Crate`, `plant:Grass` or a model's path). An entry this
//! engine doesn't know (a built-in from a newer version) is left out when
//! read, so the rest of the palette still loads. Every setting but the
//! item has a default, so a hand-written file can be short.
//!
//! An item's masks (`mask`: noise patches, slope, height and moisture) and
//! the palette's decals (`decals`, with `decal_density` and
//! `decal_spacing`: the pictures the pane's decal brush lays on the
//! ground) are written only when they're set, so a palette without them
//! is the same file it always was, and one written before they existed
//! reads as it did.

use std::io;
use std::path::{Path, PathBuf};

use axcenity_core::FoliageSource;
use axcenity_terrain::brush::{ScatterItem, ScatterMask};
use serde::{Deserialize, Serialize};

use crate::world_presets::safe_file_stem;

/// A scatter palette file's extension.
pub const SCATTER_PALETTE_EXTENSION: &str = "axscatter";
/// The newest palette format this reads, and the one it writes.
pub const SCATTER_PALETTE_FORMAT: u32 = 1;
/// A new palette's density: items per 100 square studs.
pub const DEFAULT_SCATTER_DENSITY: f32 = 8.0;
/// A new palette's spacing: studs between items.
pub const DEFAULT_SCATTER_SPACING: f32 = 1.0;
/// A new palette's decal density: decals per 100 square studs.
pub const DEFAULT_DECAL_DENSITY: f32 = 2.0;
/// A new palette's decal spacing: studs between decals' middles.
pub const DEFAULT_DECAL_SPACING: f32 = 2.0;
/// The name the ground clutter preset palette gets.
pub const GROUND_CLUTTER_PALETTE: &str = "Ground Clutter";
/// The smallest and largest a scattered decal's longer side can be, in studs.
pub const DECAL_SIZE_RANGE: (f32, f32) = (0.1, 64.0);

/// A picture the Scatter pane's decal brush lays on the ground as a
/// ProjectedDecal: a stain, a tire track, a patch of pebbles.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterDecal {
    /// The picture's path in the project, such as `assets/textures/stain.png`.
    pub texture: String,
    /// Off, it isn't placed.
    pub enabled: bool,
    /// Its share of the mix, against the other enabled decals' weights.
    pub weight: f32,
    /// The smallest and largest its longer side is, in studs.
    pub min_size: f32,
    pub max_size: f32,
    /// How wide the picture is for its height (1 is square), so a long
    /// track isn't squashed.
    pub aspect: f32,
    /// The faintest and strongest it's drawn, 0 (invisible) to 1 (solid).
    pub min_opacity: f32,
    pub max_opacity: f32,
    /// How far it's turned at random on the surface, in degrees: 0 lays
    /// every copy the same way, 360 any way.
    pub rotation: f32,
}

impl ScatterDecal {
    /// `texture` with the usual settings: on, weight 1, 3 to 6 studs
    /// across, square, a little see-through, turned any way.
    pub fn new(texture: &str) -> Self {
        Self {
            texture: texture.to_owned(),
            enabled: true,
            weight: 1.0,
            min_size: 3.0,
            max_size: 6.0,
            aspect: 1.0,
            min_opacity: 0.6,
            max_opacity: 0.9,
            rotation: 360.0,
        }
    }

    /// Whether a stroke places it: on, with a weight above 0.
    pub fn places(&self) -> bool {
        self.enabled && self.weight.is_finite() && self.weight > 0.0
    }
}

/// A scatter palette: what a `.axscatter` file holds.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterPalette {
    pub name: String,
    /// Items per 100 square studs where the brush is at full strength.
    pub density: f32,
    /// Studs kept between a new item and any painted foliage (0: none).
    pub spacing: f32,
    pub items: Vec<ScatterItem>,
    /// Decals per 100 square studs where the decal brush is at full strength.
    pub decal_density: f32,
    /// Studs kept between a new decal's middle and any other's (0: none).
    pub decal_spacing: f32,
    /// The pictures the decal brush lays on the ground.
    pub decals: Vec<ScatterDecal>,
}

impl ScatterPalette {
    /// An empty palette called `name`, at the usual density and spacing.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.trim().to_owned(),
            density: DEFAULT_SCATTER_DENSITY,
            spacing: DEFAULT_SCATTER_SPACING,
            items: Vec::new(),
            decal_density: DEFAULT_DECAL_DENSITY,
            decal_spacing: DEFAULT_DECAL_SPACING,
            decals: Vec::new(),
        }
    }

    /// The ground clutter preset, called `name`: the built-in Rock at
    /// three small sizes (gravel, pebbles and a few fist-sized stones in
    /// loose patches), each lying square to the slope and sunk a little so
    /// none floats, visual only, close together. For the litter that ties
    /// props to the ground: round a wall's foot, under a vehicle, along a
    /// road's edge. Twigs, leaves and the like need models of the
    /// project's own, added from Assets.
    pub fn ground_clutter(name: &str) -> Self {
        let rock = |weight: f32, (min_scale, max_scale): (f32, f32), sink: f32| ScatterItem {
            weight,
            min_scale,
            max_scale,
            align_to_slope: 1.0,
            sink,
            ..ScatterItem::new(FoliageSource::Prop(axcenity_core::FoliageProp::Rock))
        };
        let mut stones = rock(1.0, (0.12, 0.25), 0.12);
        stones.mask = ScatterMask {
            noise: 0.8,
            noise_scale: 14.0,
            noise_threshold: 0.55,
            ..ScatterMask::default()
        };
        Self {
            density: 90.0,
            spacing: 0.35,
            // The Rock is 5 studs across: these are a quarter of a stud
            // to a little over a stud.
            items: vec![
                rock(6.0, (0.03, 0.06), 0.02),
                rock(3.0, (0.06, 0.12), 0.05),
                stones,
            ],
            ..Self::new(name)
        }
    }

    pub fn to_json(&self) -> String {
        let file = PaletteFile {
            format: SCATTER_PALETTE_FORMAT,
            name: self.name.clone(),
            density: self.density,
            spacing: self.spacing,
            items: self.items.iter().map(ItemRecord::from).collect(),
            decal_density: self.decal_density,
            decal_spacing: self.decal_spacing,
            decals: self.decals.iter().map(DecalRecord::from).collect(),
        };
        serde_json::to_string_pretty(&file).expect("a palette is always JSON") + "\n"
    }

    pub fn from_json(text: &str) -> Result<ScatterPalette, ScatterPaletteError> {
        let file: PaletteFile = serde_json::from_str(text)?;
        if file.format > SCATTER_PALETTE_FORMAT {
            return Err(ScatterPaletteError::TooNew { found: file.format });
        }
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        Ok(ScatterPalette {
            name: file.name.trim().to_owned(),
            density: finite(file.density, DEFAULT_SCATTER_DENSITY).max(0.0),
            spacing: finite(file.spacing, DEFAULT_SCATTER_SPACING).max(0.0),
            items: file
                .items
                .into_iter()
                .filter_map(ItemRecord::into_item)
                .collect(),
            decal_density: finite(file.decal_density, DEFAULT_DECAL_DENSITY).max(0.0),
            decal_spacing: finite(file.decal_spacing, DEFAULT_DECAL_SPACING).max(0.0),
            decals: file
                .decals
                .into_iter()
                .filter_map(DecalRecord::into_decal)
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct PaletteFile {
    format: u32,
    name: String,
    #[serde(default = "default_density")]
    density: f32,
    #[serde(default = "default_spacing")]
    spacing: f32,
    #[serde(default)]
    items: Vec<ItemRecord>,
    #[serde(
        default = "default_decal_density",
        skip_serializing_if = "is_default_decal_density"
    )]
    decal_density: f32,
    #[serde(
        default = "default_decal_spacing",
        skip_serializing_if = "is_default_decal_spacing"
    )]
    decal_spacing: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    decals: Vec<DecalRecord>,
}

fn default_density() -> f32 {
    DEFAULT_SCATTER_DENSITY
}

fn default_spacing() -> f32 {
    DEFAULT_SCATTER_SPACING
}

fn default_decal_density() -> f32 {
    DEFAULT_DECAL_DENSITY
}

fn is_default_decal_density(value: &f32) -> bool {
    *value == DEFAULT_DECAL_DENSITY
}

fn default_decal_spacing() -> f32 {
    DEFAULT_DECAL_SPACING
}

fn is_default_decal_spacing(value: &f32) -> bool {
    *value == DEFAULT_DECAL_SPACING
}

/// One item as written: its settings by name, with `item` the source.
#[derive(Serialize, Deserialize)]
struct ItemRecord {
    item: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default = "one")]
    weight: f32,
    #[serde(default = "default_min_scale")]
    min_scale: f32,
    #[serde(default = "default_max_scale")]
    max_scale: f32,
    #[serde(default)]
    collide: bool,
    /// Degrees of random turn, 0 to 360.
    #[serde(default = "full_turn")]
    rotation: f32,
    #[serde(default)]
    align_to_slope: f32,
    #[serde(default)]
    sink: f32,
    /// Left out while every mask is at its default.
    #[serde(default, skip_serializing_if = "MaskRecord::is_default")]
    mask: MaskRecord,
}

/// An item's masks as written: each setting by name, any of them left out
/// for its default.
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct MaskRecord {
    noise: f32,
    noise_scale: f32,
    noise_threshold: f32,
    noise_contrast: f32,
    noise_seed: u32,
    min_slope: f32,
    max_slope: f32,
    min_height: f32,
    max_height: f32,
    moisture: f32,
    moisture_reach: f32,
}

impl Default for MaskRecord {
    fn default() -> Self {
        Self::from(&ScatterMask::default())
    }
}

impl MaskRecord {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The masks, each setting a number in its range.
    fn into_mask(self) -> ScatterMask {
        use axcenity_terrain::brush::{MOISTURE_REACH, NOISE_SCALE, OPEN_HEIGHT};
        let usual = ScatterMask::default();
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        ScatterMask {
            noise: finite(self.noise, usual.noise).clamp(0.0, 1.0),
            noise_scale: finite(self.noise_scale, usual.noise_scale)
                .clamp(NOISE_SCALE.0, NOISE_SCALE.1),
            noise_threshold: finite(self.noise_threshold, usual.noise_threshold).clamp(0.0, 1.0),
            noise_contrast: finite(self.noise_contrast, usual.noise_contrast).clamp(0.0, 1.0),
            noise_seed: self.noise_seed,
            min_slope: finite(self.min_slope, usual.min_slope).clamp(0.0, 90.0),
            max_slope: finite(self.max_slope, usual.max_slope).clamp(0.0, 90.0),
            min_height: finite(self.min_height, usual.min_height).clamp(-OPEN_HEIGHT, OPEN_HEIGHT),
            max_height: finite(self.max_height, usual.max_height).clamp(-OPEN_HEIGHT, OPEN_HEIGHT),
            moisture: finite(self.moisture, usual.moisture).clamp(-1.0, 1.0),
            moisture_reach: finite(self.moisture_reach, usual.moisture_reach)
                .clamp(MOISTURE_REACH.0, MOISTURE_REACH.1),
        }
    }
}

impl From<&ScatterMask> for MaskRecord {
    fn from(mask: &ScatterMask) -> Self {
        Self {
            noise: mask.noise,
            noise_scale: mask.noise_scale,
            noise_threshold: mask.noise_threshold,
            noise_contrast: mask.noise_contrast,
            noise_seed: mask.noise_seed,
            min_slope: mask.min_slope,
            max_slope: mask.max_slope,
            min_height: mask.min_height,
            max_height: mask.max_height,
            moisture: mask.moisture,
            moisture_reach: mask.moisture_reach,
        }
    }
}

/// One decal as written: its settings by name, with `texture` the picture.
#[derive(Serialize, Deserialize)]
struct DecalRecord {
    texture: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default = "one")]
    weight: f32,
    #[serde(default = "default_decal_min_size")]
    min_size: f32,
    #[serde(default = "default_decal_max_size")]
    max_size: f32,
    #[serde(default = "one")]
    aspect: f32,
    #[serde(default = "default_decal_min_opacity")]
    min_opacity: f32,
    #[serde(default = "default_decal_max_opacity")]
    max_opacity: f32,
    /// Degrees of random turn, 0 to 360.
    #[serde(default = "full_turn")]
    rotation: f32,
}

fn default_decal_min_size() -> f32 {
    ScatterDecal::new("").min_size
}

fn default_decal_max_size() -> f32 {
    ScatterDecal::new("").max_size
}

fn default_decal_min_opacity() -> f32 {
    ScatterDecal::new("").min_opacity
}

fn default_decal_max_opacity() -> f32 {
    ScatterDecal::new("").max_opacity
}

impl From<&ScatterDecal> for DecalRecord {
    fn from(decal: &ScatterDecal) -> Self {
        Self {
            texture: decal.texture.clone(),
            enabled: decal.enabled,
            weight: decal.weight,
            min_size: decal.min_size,
            max_size: decal.max_size,
            aspect: decal.aspect,
            min_opacity: decal.min_opacity,
            max_opacity: decal.max_opacity,
            rotation: decal.rotation,
        }
    }
}

impl DecalRecord {
    /// The decal, or `None` without a picture.
    fn into_decal(self) -> Option<ScatterDecal> {
        let texture = self.texture.trim();
        if texture.is_empty() {
            return None;
        }
        let usual = ScatterDecal::new(texture);
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        let size = |value: f32, default: f32| {
            finite(value, default).clamp(DECAL_SIZE_RANGE.0, DECAL_SIZE_RANGE.1)
        };
        Some(ScatterDecal {
            enabled: self.enabled,
            weight: finite(self.weight, 1.0).max(0.0),
            min_size: size(self.min_size, usual.min_size),
            max_size: size(self.max_size, usual.max_size),
            aspect: finite(self.aspect, 1.0).clamp(0.05, 20.0),
            min_opacity: finite(self.min_opacity, usual.min_opacity).clamp(0.0, 1.0),
            max_opacity: finite(self.max_opacity, usual.max_opacity).clamp(0.0, 1.0),
            rotation: finite(self.rotation, 360.0).clamp(0.0, 360.0),
            ..usual
        })
    }
}

fn yes() -> bool {
    true
}

fn one() -> f32 {
    1.0
}

fn default_min_scale() -> f32 {
    ScatterItem::new(FoliageSource::Model(String::new())).min_scale
}

fn default_max_scale() -> f32 {
    ScatterItem::new(FoliageSource::Model(String::new())).max_scale
}

fn full_turn() -> f32 {
    360.0
}

impl From<&ScatterItem> for ItemRecord {
    fn from(item: &ScatterItem) -> Self {
        Self {
            item: item.source.entry(),
            enabled: item.enabled,
            weight: item.weight,
            min_scale: item.min_scale,
            max_scale: item.max_scale,
            collide: item.collide,
            rotation: item.rotation,
            align_to_slope: item.align_to_slope,
            sink: item.sink,
            mask: MaskRecord::from(&item.mask),
        }
    }
}

impl ItemRecord {
    /// The item, or `None` for an empty or unknown entry.
    fn into_item(self) -> Option<ScatterItem> {
        let source = FoliageSource::from_entry(&self.item)?;
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        Some(ScatterItem {
            source,
            enabled: self.enabled,
            weight: finite(self.weight, 1.0).max(0.0),
            min_scale: finite(self.min_scale, 1.0),
            max_scale: finite(self.max_scale, 1.0),
            collide: self.collide,
            rotation: finite(self.rotation, 360.0).clamp(0.0, 360.0),
            align_to_slope: finite(self.align_to_slope, 0.0).clamp(0.0, 1.0),
            sink: finite(self.sink, 0.0),
            mask: self.mask.into_mask(),
        })
    }
}

/// Why a scatter palette couldn't be read or saved.
#[derive(Debug, thiserror::Error)]
pub enum ScatterPaletteError {
    #[error("not a scatter palette: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "scatter palette format {found} is newer than this engine understands; update Axcenity to use it"
    )]
    TooNew { found: u32 },
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// A palette file found on disk, and what it holds.
pub type FoundPalette = (PathBuf, ScatterPalette);
/// A palette file that couldn't be read, and why.
pub type PaletteProblem = (PathBuf, ScatterPaletteError);

/// A file name for a palette called `name`, such as `Forest Floor.axscatter`
/// (characters file systems don't allow become `-`).
pub fn scatter_palette_file_name(name: &str) -> String {
    format!(
        "{}.{SCATTER_PALETTE_EXTENSION}",
        safe_file_stem(name, "Palette")
    )
}

/// Saves `palette` in `folder` (made if missing), named after it, replacing
/// a palette file of the same name. Returns the file.
pub fn save_scatter_palette(
    folder: &Path,
    palette: &ScatterPalette,
) -> Result<PathBuf, ScatterPaletteError> {
    std::fs::create_dir_all(folder)?;
    let file = folder.join(scatter_palette_file_name(&palette.name));
    std::fs::write(&file, palette.to_json())?;
    Ok(file)
}

/// The scatter palettes in `folder`, sorted by name, and the files that
/// couldn't be read. A missing folder has none.
pub fn list_scatter_palettes(folder: &Path) -> (Vec<FoundPalette>, Vec<PaletteProblem>) {
    let mut found = Vec::new();
    let mut problems = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return (found, problems);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(SCATTER_PALETTE_EXTENSION) {
            continue;
        }
        match std::fs::read_to_string(&path)
            .map_err(ScatterPaletteError::from)
            .and_then(|text| ScatterPalette::from_json(&text))
        {
            Ok(palette) => found.push((path, palette)),
            Err(problem) => problems.push((path, problem)),
        }
    }
    found.sort_by_key(|(_, palette)| palette.name.to_lowercase());
    (found, problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_core::{FoliagePlant, FoliageProp};

    fn palette() -> ScatterPalette {
        let mut palette = ScatterPalette::new("Forest Floor");
        palette.density = 12.5;
        palette.spacing = 2.0;
        palette.items = vec![
            ScatterItem {
                weight: 3.0,
                min_scale: 0.5,
                max_scale: 2.0,
                collide: true,
                rotation: 45.0,
                align_to_slope: 0.75,
                sink: 0.3,
                ..ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock))
            },
            ScatterItem {
                enabled: false,
                ..ScatterItem::new(FoliageSource::Plant(FoliagePlant::Grass))
            },
            ScatterItem::new(FoliageSource::Model("assets/models/Fern.glb".into())),
        ];
        palette
    }

    #[test]
    fn a_palette_round_trips_through_its_file() {
        let palette = palette();
        let text = palette.to_json();
        assert!(text.contains("\"prop:Rock\""), "{text}");
        assert!(text.contains("\"assets/models/Fern.glb\""), "{text}");
        assert_eq!(ScatterPalette::from_json(&text).unwrap(), palette);
    }

    #[test]
    fn a_short_file_takes_defaults_and_unknown_items_are_left_out() {
        let text = r#"{
  "format": 1,
  "name": "Rocks",
  "items": [
    { "item": "prop:Rock" },
    { "item": "prop:Unicorn" },
    { "item": "" },
    { "item": "plant:Bush", "weight": 2, "rotation": 720, "align_to_slope": 4 }
  ]
}"#;
        let read = ScatterPalette::from_json(text).unwrap();
        assert_eq!(read.density, DEFAULT_SCATTER_DENSITY);
        assert_eq!(read.spacing, DEFAULT_SCATTER_SPACING);
        assert_eq!(read.items.len(), 2);
        assert_eq!(
            read.items[0],
            ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock))
        );
        assert_eq!(read.items[1].weight, 2.0);
        assert_eq!(read.items[1].rotation, 360.0);
        assert_eq!(read.items[1].align_to_slope, 1.0);
        let newer = text.replace("\"format\": 1", "\"format\": 9");
        assert!(matches!(
            ScatterPalette::from_json(&newer),
            Err(ScatterPaletteError::TooNew { found: 9 })
        ));
    }

    /// A palette file exactly as the engine wrote it before masks and
    /// decals existed.
    const OLD_FILE: &str = r#"{
  "format": 1,
  "name": "Scrub",
  "density": 12.5,
  "spacing": 2.0,
  "items": [
    {
      "item": "prop:Rock",
      "enabled": true,
      "weight": 3.0,
      "min_scale": 0.5,
      "max_scale": 2.0,
      "collide": true,
      "rotation": 45.0,
      "align_to_slope": 0.75,
      "sink": 0.3
    },
    {
      "item": "plant:Grass",
      "enabled": false,
      "weight": 1.0,
      "min_scale": 0.85,
      "max_scale": 1.15,
      "collide": false,
      "rotation": 360.0,
      "align_to_slope": 0.0,
      "sink": 0.0
    }
  ]
}
"#;

    #[test]
    fn an_old_palette_reads_with_open_masks_and_no_decals_and_saves_as_the_same_file() {
        let read = ScatterPalette::from_json(OLD_FILE).unwrap();
        assert_eq!(read.items.len(), 2);
        assert!(read.items.iter().all(|item| item.mask.is_open()));
        assert!(
            read.items
                .iter()
                .all(|item| item.mask == ScatterMask::default())
        );
        assert!(read.decals.is_empty());
        assert_eq!(read.decal_density, DEFAULT_DECAL_DENSITY);
        assert_eq!(read.decal_spacing, DEFAULT_DECAL_SPACING);
        // Byte for byte: nothing new is written until it's set.
        assert_eq!(read.to_json(), OLD_FILE);
    }

    #[test]
    fn masks_and_decals_round_trip_and_odd_values_are_put_in_range() {
        let mut palette = palette();
        palette.items[0].mask = ScatterMask {
            noise: 0.8,
            noise_scale: 64.0,
            noise_threshold: 0.35,
            noise_contrast: 0.9,
            noise_seed: 42,
            min_slope: 5.0,
            max_slope: 30.0,
            min_height: -12.0,
            max_height: 80.0,
            moisture: -0.75,
            moisture_reach: 24.0,
        };
        palette.decal_density = 5.0;
        palette.decal_spacing = 0.5;
        palette.decals = vec![
            ScatterDecal {
                weight: 2.0,
                min_size: 1.0,
                max_size: 2.5,
                aspect: 0.25,
                min_opacity: 0.3,
                max_opacity: 0.5,
                rotation: 20.0,
                ..ScatterDecal::new("assets/textures/tracks.png")
            },
            ScatterDecal {
                enabled: false,
                ..ScatterDecal::new("assets/textures/stain.png")
            },
        ];
        let text = palette.to_json();
        assert!(text.contains("\"mask\""), "{text}");
        // Only the masked item writes its masks.
        assert_eq!(text.matches("\"mask\"").count(), 1, "{text}");
        assert!(text.contains("\"assets/textures/tracks.png\""), "{text}");
        assert_eq!(ScatterPalette::from_json(&text).unwrap(), palette);

        // A hand-written file: a mask with one setting takes the rest as
        // defaults, and numbers out of range are brought in.
        let text = r#"{
  "format": 1,
  "name": "Hand",
  "items": [
    { "item": "plant:Bush", "mask": { "moisture": 5 } },
    { "item": "plant:Grass", "mask": { "noise": 2, "noise_scale": 0, "max_slope": 400 } }
  ],
  "decals": [
    { "texture": "assets/textures/stain.png" },
    { "texture": "  " },
    { "texture": "assets/textures/big.png", "min_size": 900, "max_opacity": 3, "rotation": -5 }
  ]
}"#;
        let read = ScatterPalette::from_json(text).unwrap();
        assert_eq!(
            read.items[0].mask,
            ScatterMask {
                moisture: 1.0,
                ..ScatterMask::default()
            }
        );
        assert_eq!(read.items[1].mask.noise, 1.0);
        assert_eq!(read.items[1].mask.noise_scale, 1.0);
        assert_eq!(read.items[1].mask.max_slope, 90.0);
        assert_eq!(
            read.decals.len(),
            2,
            "a decal without a picture is left out"
        );
        assert_eq!(
            read.decals[0],
            ScatterDecal::new("assets/textures/stain.png")
        );
        assert_eq!(read.decals[1].min_size, DECAL_SIZE_RANGE.1);
        assert_eq!(read.decals[1].max_opacity, 1.0);
        assert_eq!(read.decals[1].rotation, 0.0);
    }

    #[test]
    fn the_ground_clutter_preset_is_small_rocks_that_lie_to_the_slope_and_sink() {
        let clutter = ScatterPalette::ground_clutter(GROUND_CLUTTER_PALETTE);
        assert_eq!(clutter.name, "Ground Clutter");
        assert_eq!(clutter.items.len(), 3);
        for item in &clutter.items {
            assert_eq!(item.source, FoliageSource::Prop(FoliageProp::Rock));
            assert!(item.places());
            assert_eq!(item.align_to_slope, 1.0);
            assert!(item.sink > 0.0);
            assert!(!item.collide);
            // The Rock is 5 studs across: nothing here is over 1.25.
            assert!(item.max_scale <= 0.25, "{}", item.max_scale);
        }
        assert!(clutter.items[2].mask.has_noise());
        assert_eq!(
            ScatterPalette::from_json(&clutter.to_json()).unwrap(),
            clutter
        );
    }

    #[test]
    fn palettes_save_by_name_and_list_sorted_with_problems_apart() {
        let folder =
            std::env::temp_dir().join(format!("axcenity-scatter-palettes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(list_scatter_palettes(&folder).0.len(), 0);
        let forest = palette();
        let file = save_scatter_palette(&folder, &forest).unwrap();
        assert_eq!(file, folder.join("Forest Floor.axscatter"));
        let mut beach = ScatterPalette::new("beach: rocks?");
        beach
            .items
            .push(ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock)));
        let beach_file = save_scatter_palette(&folder, &beach).unwrap();
        assert_eq!(beach_file, folder.join("beach- rocks-.axscatter"));
        std::fs::write(folder.join("Broken.axscatter"), "not json").unwrap();
        std::fs::write(folder.join("Island.axworld"), "{}").unwrap();
        let (found, problems) = list_scatter_palettes(&folder);
        let names: Vec<&str> = found.iter().map(|(_, p)| p.name.as_str()).collect();
        assert_eq!(names, ["beach: rocks?", "Forest Floor"]);
        assert_eq!(found[1].1, forest);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].0.ends_with("Broken.axscatter"));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
